use ::rocket::async_main;
use anyhow::Result;
use std::{path::PathBuf, time::Duration};

use clap::Parser;
use espflash::{elf::FirmwareImageBuilder, Chip, FlashSize, PartitionTable};
use rocket::{response::content, State, serde::json::Json};
use serde::Serialize;

#[macro_use]
extern crate rocket;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// chip name
    #[arg(short, long)]
    chip: Chip,

    /// path to bootloader
    #[arg(short, long)]
    bootloader: Option<PathBuf>,

    /// path to partition table csv
    #[arg(short, long)]
    partition_table: Option<PathBuf>,

    /// flash size (examples: 2MB, 4MB, 8MB, 16MB)
    #[arg(short, long, default_value = "4MB")]
    flash_size: String,

    elf: PathBuf,
}

#[get("/bootloader.bin")]
fn bootloader(data: &State<PartsData>) -> Vec<u8> {
    data.bootloader.clone()
}

#[get("/partitions.bin")]
fn partitions(data: &State<PartsData>) -> Vec<u8> {
    data.partitions.clone()
}

#[get("/firmware.bin")]
fn firmware(data: &State<PartsData>) -> Vec<u8> {
    data.firmware.clone()
}

#[derive(Serialize)]
struct FirmwareInfo {
    chip: String,
    total_size: usize,
    bootloader_size: usize,
    partitions_size: usize,
    firmware_size: usize,
    flash_size: String,
}

#[get("/info")]
fn info(data: &State<PartsData>) -> Json<FirmwareInfo> {
    Json(FirmwareInfo {
        chip: data.chip.clone(),
        total_size: data.total_size,
        bootloader_size: data.bootloader_size,
        partitions_size: data.partitions_size,
        firmware_size: data.firmware_size,
        flash_size: data.flash_size.clone(),
    })
}

#[get("/")]
fn index() -> content::RawHtml<&'static str> {
    content::RawHtml(
        r#"
        <html>
        <head>
            <title>ESP Web Flasher</title>
            <style>
                body {
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                    max-width: 900px;
                    margin: 0 auto;
                    padding: 20px;
                    background-color: #f5f5f5;
                    color: #333;
                }
                h1 {
                    color: #2c3e50;
                    margin-bottom: 30px;
                    font-weight: 300;
                    font-size: 2.5em;
                }
                h3 {
                    color: #34495e;
                    margin-bottom: 15px;
                    font-weight: 400;
                }
                .main-container {
                    background-color: white;
                    border-radius: 10px;
                    padding: 30px;
                    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                }
                .info-box {
                    background-color: #f8f9fa;
                    border: 1px solid #e9ecef;
                    border-radius: 8px;
                    padding: 20px;
                    margin: 20px 0;
                }
                .info-grid {
                    display: grid;
                    grid-template-columns: repeat(2, 1fr);
                    gap: 15px;
                }
                .info-item {
                    padding: 8px 0;
                    border-bottom: 1px solid #eee;
                }
                .info-item:last-child {
                    border-bottom: none;
                }
                .size-label {
                    font-weight: 600;
                    color: #666;
                    display: inline-block;
                    width: 140px;
                }
                .size-value {
                    color: #2c3e50;
                    font-weight: 400;
                }
                .total-row {
                    margin-top: 15px;
                    padding-top: 15px;
                    border-top: 2px solid #dee2e6;
                    font-size: 1.1em;
                }
                #console {
                    background-color: #1e1e1e;
                    color: #d4d4d4;
                    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
                    font-size: 13px;
                    padding: 15px;
                    border-radius: 8px;
                    height: 250px;
                    overflow-y: auto;
                    margin-top: 20px;
                    white-space: pre-wrap;
                    line-height: 1.5;
                    box-shadow: inset 0 2px 4px rgba(0,0,0,0.2);
                }
                .progress-info {
                    background-color: #e3f2fd;
                    border: 1px solid #90caf9;
                    border-radius: 8px;
                    padding: 15px;
                    margin: 20px 0;
                    font-family: monospace;
                }
                .progress-info div {
                    margin: 5px 0;
                }
                esp-web-install-button {
                    margin: 20px 0;
                }
                button {
                    background-color: #3498db;
                    color: white;
                    border: none;
                    padding: 10px 20px;
                    border-radius: 5px;
                    font-size: 14px;
                    cursor: pointer;
                    transition: background-color 0.3s;
                }
                button:hover {
                    background-color: #2980b9;
                }
                button:active {
                    transform: translateY(1px);
                }
                .button-group {
                    margin-top: 20px;
                    display: flex;
                    gap: 10px;
                }
                .note {
                    background-color: #fff3cd;
                    border: 1px solid #ffeaa7;
                    color: #856404;
                    padding: 12px;
                    border-radius: 5px;
                    margin: 15px 0;
                    font-size: 0.9em;
                }
                .error-message {
                    background-color: #f8d7da;
                    border: 1px solid #f5c6cb;
                    color: #721c24;
                    padding: 20px;
                    border-radius: 8px;
                    text-align: center;
                }
            </style>
        </head>
        <body>
            <h1>ESP Web Flasher</h1>

            <div id="main" class="main-container" style="display: none;">
                <div id="firmwareInfo" class="info-box" style="display: none;">
                    <h3>Firmware Information</h3>
                    <div class="info-grid">
                        <div>
                            <div class="info-item">
                                <span class="size-label">Chip:</span>
                                <span id="chipType" class="size-value"></span>
                            </div>
                            <div class="info-item">
                                <span class="size-label">Flash Size:</span>
                                <span id="flashSize" class="size-value"></span>
                            </div>
                        </div>
                        <div>
                            <div class="info-item">
                                <span class="size-label">Bootloader:</span>
                                <span id="bootloaderSize" class="size-value"></span>
                            </div>
                            <div class="info-item">
                                <span class="size-label">Partitions:</span>
                                <span id="partitionsSize" class="size-value"></span>
                            </div>
                            <div class="info-item">
                                <span class="size-label">Firmware:</span>
                                <span id="firmwareSize" class="size-value"></span>
                            </div>
                        </div>
                    </div>
                    <div class="total-row">
                        <span class="size-label">Total Size:</span>
                        <span id="totalSize" class="size-value"></span>
                    </div>
                </div>

                <script type="module" src="https://unpkg.com/esp-web-tools@9.4.3/dist/web/install-button.js?module">
                </script>
                <esp-web-install-button id="installButton" manifest="manifest.json"></esp-web-install-button>
                
                <div class="note">
                    <strong>Note:</strong> Make sure to close any applications using your device's COM port (e.g., Serial Monitor)
                </div>
                
                <div class="progress-info" id="progressInfo" style="display: none;">
                    <div><strong>Progress:</strong> <span id="progressPercent">0%</span></div>
                    <div><strong>Uploaded:</strong> <span id="uploadedBytes">0</span> / <span id="totalBytes">0</span> bytes</div>
                </div>

                <h3>Console Output</h3>
                <div id="console"></div>
                
                <div class="button-group">
                    <button onclick="downloadLogs()">Download Logs</button>
                    <button onclick="clearLogs()">Clear Logs</button>
                </div>
            </div>
            
            <div id="notSupported" class="main-container error-message" style="display: none;">
                <h2>Browser Not Supported</h2>
                <p>Your browser does not support the Web Serial API.</p>
                <p>Please use Chrome or Microsoft Edge to flash your ESP device.</p>
            </div>

            <script>
                function formatBytes(bytes) {
                    if (bytes === 0) return '0 Bytes';
                    const k = 1024;
                    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
                    const i = Math.floor(Math.log(bytes) / Math.log(k));
                    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
                }

                function log(message, type = 'info') {
                    const console = document.getElementById('console');
                    const timestamp = new Date().toLocaleTimeString();
                    const logEntry = document.createElement('div');
                    
                    let color = '#d4d4d4';
                    if (type === 'error') color = '#f48771';
                    else if (type === 'success') color = '#98c379';
                    else if (type === 'warning') color = '#e5c07b';
                    else if (type === 'progress') color = '#61afef';
                    
                    logEntry.style.color = color;
                    logEntry.textContent = `[${timestamp}] ${message}`;
                    console.appendChild(logEntry);
                    console.scrollTop = console.scrollHeight;
                }

                function downloadLogs() {
                    const logs = document.getElementById('console').textContent;
                    const blob = new Blob([logs], { type: 'text/plain' });
                    const url = window.URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = `esp-flash-logs-${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.txt`;
                    a.click();
                    window.URL.revokeObjectURL(url);
                }

                function clearLogs() {
                    document.getElementById('console').innerHTML = '';
                    log('Logs cleared', 'info');
                }

                async function fetchFirmwareInfo() {
                    try {
                        const response = await fetch('/info');
                        const info = await response.json();
                        
                        document.getElementById('chipType').textContent = info.chip;
                        document.getElementById('flashSize').textContent = info.flash_size;
                        document.getElementById('bootloaderSize').textContent = formatBytes(info.bootloader_size);
                        document.getElementById('partitionsSize').textContent = formatBytes(info.partitions_size);
                        document.getElementById('firmwareSize').textContent = formatBytes(info.firmware_size);
                        document.getElementById('totalSize').textContent = formatBytes(info.total_size);
                        document.getElementById('firmwareInfo').style.display = 'block';
                        
                        log('Firmware information loaded', 'success');
                        log(`Total size to flash: ${formatBytes(info.total_size)}`, 'info');
                    } catch (error) {
                        log('Failed to fetch firmware information: ' + error, 'error');
                    }
                }

                if (navigator.serial) {
                    document.getElementById("notSupported").style.display = 'none';
                    document.getElementById("main").style.display = 'block';
                    
                    // Fetch firmware info when page loads
                    fetchFirmwareInfo();
                    
                    // Listen for esp-web-tools events
                    const installButton = document.getElementById('installButton');
                    
                    installButton.addEventListener('state-changed', (e) => {
                        const state = e.detail;
                        log(`State changed: ${state.state}`);
                        
                        if (state.state === 'initializing') {
                            log('Initializing connection...');
                            if (state.details) {
                                log(`Port: ${state.details.port || 'Auto-detecting'}`);
                            }
                        } else if (state.state === 'manifest') {
                            log('Loading manifest...');
                        } else if (state.state === 'preparing') {
                            log('Preparing installation...');
                            if (state.chipFamily) {
                                log(`Detected chip family: ${state.chipFamily}`);
                            }
                        } else if (state.state === 'erasing') {
                            log('Erasing device...', 'warning');
                        } else if (state.state === 'writing') {
                            log('Writing firmware...', 'progress');
                            document.getElementById('progressInfo').style.display = 'block';
                            
                            // Update progress with byte information if available
                            if (state.details) {
                                const { bytesWritten, bytesTotal, percentage } = state.details;
                                document.getElementById('progressPercent').textContent = Math.round(percentage) + '%';
                                document.getElementById('uploadedBytes').textContent = formatBytes(bytesWritten);
                                document.getElementById('totalBytes').textContent = formatBytes(bytesTotal);
                                
                                // Log progress every 10%
                                if (percentage % 10 === 0) {
                                    log(`Progress: ${Math.round(percentage)}% - ${formatBytes(bytesWritten)} / ${formatBytes(bytesTotal)}`, 'progress');
                                }
                            }
                        } else if (state.state === 'finished') {
                            log('Installation complete!', 'success');
                            log('Device will restart with new firmware.', 'success');
                        } else if (state.state === 'error') {
                            log(`Error: ${state.message}`, 'error');
                            if (state.details) {
                                log(`Error details: ${JSON.stringify(state.details)}`, 'error');
                            }
                        }
                    });
                    
                } else {
                    document.getElementById("notSupported").style.display = 'block';
                    document.getElementById("main").style.display = 'none';
                }
            </script>

        </body>
        </html>
        "#,
    )
}

#[get("/manifest.json")]
fn manifest() -> content::RawJson<&'static str> {
    content::RawJson(
        r#"
        {
            "name": "ESP Application",
            "new_install_prompt_erase": true,
            "builds": [
                {
                "chipFamily": "ESP32",
                "parts": [
                    {
                    "path": "bootloader.bin",
                    "offset": 4096
                    },
                    {
                    "path": "partitions.bin",
                    "offset": 32768
                    },
                    {
                    "path": "firmware.bin",
                    "offset": 65536
                    }
                ]
                },
                {
                "chipFamily": "ESP32-C3",
                "parts": [
                    {
                    "path": "bootloader.bin",
                    "offset": 0
                    },
                    {
                    "path": "partitions.bin",
                    "offset": 32768
                    },
                    {
                    "path": "firmware.bin",
                    "offset": 65536
                    }
                ]
                },
                {
                "chipFamily": "ESP32-S2",
                "parts": [
                    {
                    "path": "bootloader.bin",
                    "offset": 4096
                    },
                    {
                    "path": "partitions.bin",
                    "offset": 32768
                    },
                    {
                    "path": "firmware.bin",
                    "offset": 65536
                    }
                ]
                },
                {
                "chipFamily": "ESP32-S3",
                "parts": [
                    {
                    "path": "bootloader.bin",
                    "offset": 0
                    },
                    {
                    "path": "partitions.bin",
                    "offset": 32768
                    },
                    {
                    "path": "firmware.bin",
                    "offset": 65536
                    }
                ]
                }
            ]
        }
        "#,
    )
}

struct PartsData {
    chip: String,
    bootloader: Vec<u8>,
    partitions: Vec<u8>,
    firmware: Vec<u8>,
    total_size: usize,
    bootloader_size: usize,
    partitions_size: usize,
    firmware_size: usize,
    flash_size: String,
}

fn prepare() -> Result<PartsData> {
    let opts = Args::parse();

    // Display file information
    let elf_metadata = std::fs::metadata(&opts.elf)?;
    println!("ELF file: {}", opts.elf.display());
    println!("  Size: {} bytes", elf_metadata.len());

    let elf = std::fs::read(opts.elf)?;

    let p = if let Some(p) = &opts.partition_table {
        Some(PartitionTable::try_from_bytes(std::fs::read(p)?)?)
    } else {
        None
    };

    let b = if let Some(p) = &opts.bootloader {
        Some(std::fs::read(p)?)
    } else {
        None
    };

    let flash_size = match opts.flash_size.to_uppercase().as_str() {
        "2MB" => FlashSize::Flash2Mb,
        "4MB" => FlashSize::Flash4Mb,
        "8MB" => FlashSize::Flash8Mb,
        "16MB" => FlashSize::Flash16Mb,
        _ => {
            eprintln!("Warning: Unknown flash size '{}', defaulting to 4MB", opts.flash_size);
            FlashSize::Flash4Mb
        }
    };

    let firmware = FirmwareImageBuilder::new(&elf)
        .flash_size(Some(flash_size))
        .build()?;

    let chip = opts.chip;
    let chip_name = match chip {
        Chip::Esp32 => "ESP32",
        Chip::Esp32c3 => "ESP32-C3",
        Chip::Esp32s2 => "ESP32-S2",
        Chip::Esp32s3 => "ESP32-S3",
        Chip::Esp8266 => "ESP8266",
    };

    let image = chip.get_flash_image(&firmware, b, p, None, None)?;
    let parts: Vec<_> = image.flash_segments().collect();
    let bootloader = &parts[0];
    let partitions = &parts[1];
    let app = &parts[2];

    let bootloader_data = bootloader.data.to_vec();
    let partitions_data = partitions.data.to_vec();
    let firmware_data = app.data.to_vec();
    
    let bootloader_size = bootloader_data.len();
    let partitions_size = partitions_data.len();
    let firmware_size = firmware_data.len();
    let total_size = bootloader_size + partitions_size + firmware_size;

    println!("Firmware prepared:");
    println!("  Chip: {}", chip_name);
    println!("  Flash size: {}", opts.flash_size);
    println!("  Bootloader: {} bytes", bootloader_size);
    println!("  Partitions: {} bytes", partitions_size);
    println!("  Firmware: {} bytes", firmware_size);
    println!("  Total: {} bytes", total_size);

    Ok(PartsData {
        chip: chip_name.to_string(),
        bootloader: bootloader_data,
        partitions: partitions_data,
        firmware: firmware_data,
        total_size,
        bootloader_size,
        partitions_size,
        firmware_size,
        flash_size: opts.flash_size.clone(),
    })
}

fn main() -> Result<()> {
    let data = prepare()?;

    println!("\nStarting web server...");
    println!("Server will be available at: http://127.0.0.1:8000/");
    println!("Opening browser automatically in 1 second...\n");

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(1000));
        opener::open_browser("http://127.0.0.1:8000/").ok();
    });

    async_main(async move {
        let _res = rocket::build()
            .mount(
                "/",
                routes![index, manifest, bootloader, partitions, firmware, info],
            )
            .manage(data)
            .launch()
            .await
            .expect("Problem launching server");
    });

    Ok(())
}
