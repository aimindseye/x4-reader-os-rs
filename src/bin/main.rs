// hardware init, construct Kernel + AppManager, boot, run
#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::ram;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use static_cell::{ConstStaticCell, StaticCell};
use x4_os::app::AppShell;
use x4_os::apps::Launcher;
use x4_os::apps::files::FilesApp;
use x4_os::apps::home::HomeApp;
use x4_os::apps::manager::AppManager;
use x4_os::apps::reader::ReaderApp;
use x4_os::apps::settings::SettingsApp;
use x4_os::apps::widgets::{ButtonFeedback, QuickMenu};
use x4_os::board::action::ButtonMapper;
use x4_os::board::{Board, speed_up_spi};
use x4_os::drivers::battery;
use x4_os::drivers::input::InputDriver;
use x4_os::drivers::sdcard::SdStorage;
use x4_os::drivers::storage;
use x4_os::drivers::strip::StripBuffer;
use x4_os::kernel::BookmarkCache;
use x4_os::kernel::BootConsole;
use x4_os::kernel::Kernel;
use x4_os::kernel::dir_cache::DirCache;
use x4_os::kernel::tasks;
use x4_os::kernel::work_queue;
use x4_os::ui::paint_stack;

esp_bootloader_esp_idf::esp_app_desc!();

// heavy statics: kept out of the async future to keep it ~200 B
static STRIP: ConstStaticCell<StripBuffer> = ConstStaticCell::new(StripBuffer::new());
static READER: ConstStaticCell<ReaderApp> = ConstStaticCell::new(ReaderApp::new());
static LAUNCHER: ConstStaticCell<Launcher> = ConstStaticCell::new(Launcher::new());
static QUICK_MENU: ConstStaticCell<QuickMenu> = ConstStaticCell::new(QuickMenu::new());
static BUMPS: ConstStaticCell<ButtonFeedback> = ConstStaticCell::new(ButtonFeedback::new());
static DIR_CACHE: ConstStaticCell<DirCache> = ConstStaticCell::new(DirCache::new());
static BM_CACHE: ConstStaticCell<BookmarkCache> = ConstStaticCell::new(BookmarkCache::new());

// BootConsole is heap-allocated during boot and dropped after display,
// reclaiming ~3 KB that would otherwise sit unused in .bss forever.
static HOME: StaticCell<HomeApp> = StaticCell::new();
static FILES: StaticCell<FilesApp> = StaticCell::new();
static SETTINGS: StaticCell<SettingsApp> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    paint_stack();

    // 108 KB main DRAM heap; leaves ~56 KB for stack
    esp_alloc::heap_allocator!(size: 110_592);

    // reclaim ~64 KB from 2nd-stage bootloader; net heap ~172 KB
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64_000);

    let app_shell = AppShell::new();
    info!("app shell initialised: {:?}", app_shell.screen());

    let mut console = alloc::boxed::Box::new(BootConsole::new());
    console.push("x4-os 0.1.0");
    console.push("esp32c3 rv32imc 160mhz");
    console.push("heap: 172K (108K + 64K reclaimed)");
    console.push("app shell: home/browser/reader scaffold");

    info!("booting...");

    // Safety: TIMG0 and SW_INTERRUPT are cloned here and consumed by
    // esp_rtos::start. They are never used again after this point.
    //
    // Board::init (which takes ownership of `peripherals`) does not
    // touch TIMG0 or SW_INTERRUPT, see the pin ownership table in
    // board/mod.rs for the full split.
    let timg0 = TimerGroup::new(unsafe { peripherals.TIMG0.clone_unchecked() });
    let sw_ints =
        SoftwareInterruptControl::new(unsafe { peripherals.SW_INTERRUPT.clone_unchecked() });
    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    // Peripherals move into Board::init, which splits them across
    // init_input (ADC pins, GPIO3, IO_MUX) and init_spi_peripherals
    // (SPI2, DMA, display + SD GPIOs). Each peripheral is used in
    // exactly one place, see the ownership table in board/mod.rs.
    let board = Board::init(peripherals);

    console.push("spi: dma ch0, 4096B tx+rx");

    let mut epd = board.display.epd;
    let mut delay = Delay::new();
    epd.init(&mut delay);
    console.push("epd: ssd1677 800x480 init");

    speed_up_spi();
    console.push("spi: 400kHz -> 20MHz");

    let sd = match board.storage.sd_card {
        Some(card) => {
            console.push("sd: card detected");
            SdStorage::mount(card).await
        }
        None => {
            console.push("sd: not found");
            SdStorage::empty()
        }
    };

    let sd_ok = sd.probe_ok();
    if sd_ok {
        console.push("sd: fat32 mounted");
        if let Err(e) = storage::ensure_x4_dir_async(&sd).await {
            console.push("sd: x4 dir failed");
            log::warn!("ensure_X4_DIR: {:?}", e);
        }
    }

    let mut input = InputDriver::new(board.input);
    let battery_mv = battery::adc_to_battery_mv(input.read_battery_mv());

    let mut kernel = Kernel::new(
        sd,
        epd,
        STRIP.take(),
        DIR_CACHE.take(),
        BM_CACHE.take(),
        delay,
        sd_ok,
        battery_mv,
    );

    let mut app_mgr = AppManager::new(
        LAUNCHER.take(),
        HOME.init(HomeApp::new()),
        FILES.init(FilesApp::new()),
        READER.take(),
        SETTINGS.init(SettingsApp::new()),
        app_shell,
        QUICK_MENU.take(),
        BUMPS.take(),
        ButtonMapper::new(),
    );

    info!(
        "app shell bound into app manager: {:?}",
        app_mgr.app_shell().screen()
    );

    console.push("kernel: constructed");

    kernel.show_boot_console(&console).await;
    drop(console); // reclaim ~3 KB of heap

    kernel.boot(&mut app_mgr).await;

    // register the image decoder so the kernel's worker task can
    // decode JPEG/PNG without depending on smol-epub directly
    work_queue::register_image_decoder(|data, is_jpeg, max_w, max_h| {
        let raw = if is_jpeg {
            smol_epub::jpeg::decode_jpeg_fit(data, max_w, max_h)
        } else {
            smol_epub::png::decode_png_fit(data, max_w, max_h)
        };

        raw.map(|img| work_queue::DecodedImage {
            width: img.width,
            height: img.height,
            data: img.data,
            stride: img.stride,
        })
    });

    spawner
        .spawn(tasks::input_task(input))
        .expect("spawn input_task");
    spawner
        .spawn(tasks::housekeeping_task())
        .expect("spawn housekeeping_task");
    spawner
        .spawn(tasks::idle_timeout_task())
        .expect("spawn idle_timeout_task");
    spawner
        .spawn(work_queue::worker_task())
        .expect("spawn worker_task");

    info!("kernel ready.");
    kernel.run(&mut app_mgr).await
}
