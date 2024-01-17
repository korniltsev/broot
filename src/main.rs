use std::io::Write;
use std::time::Duration;
use cli_log::*;

use reqwest::{Client, Error, Response, Url};
use log::warn;

// 0.11.14
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
#[export_name = "malloc_conf"]
pub static malloc_conf: &[u8] = b"prof:true,prof_active:true,lg_prof_sample:18\0";

struct pyroscope_config {
    url : String,
    application_name : String,
}
async fn pyroscope_loop(cfg :pyroscope_config) {
    {
        jemalloc_pprof::activate_jemalloc_profiling().await;
    }
    let client = reqwest::Client::new();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        {
            //todo termination
            let Some(ctl) = jemalloc_pprof::PROF_CTL.as_ref() else {
                warn!("Failed to get pprof ctl");
                continue;
            };
            let mut ctl = ctl.lock().await;
            let Ok(pprof) = ctl.dump_pprof() else {
                warn!("Failed to dump pprof data");
                continue;
            };


            let res = client
                .post(&cfg.url)
                .header("Content-Type", "application/octet-stream")
                .query(&[
                    ("name", cfg.application_name.as_str()),
                    ("format", "pprof"),
                    ("sampleRate", &format!("{}", 100)),
                    ("spyName", "pprofrs"),
                ])
                .body(pprof)
                .timeout(Duration::from_secs(10))
                .send().await;

            match res {
                Ok(res) => {
                    warn!("pprof data sent: {:?}", res);
                }
                Err(err) => {
                    warn!("failed to send pprof data: {:?}", err);
                }
            }
        }
    }
}


fn main() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(pyroscope_loop(
            pyroscope_config{ url: "http://localhost:4040/ingest".to_string(), application_name: "broot-rust".to_string() }
        ))
    });

    init_cli_log!();
    debug!("env::args(): {:#?}", std::env::args().collect::<Vec<String>>());
    match broot::cli::run() {
        Ok(Some(launchable)) => {
            debug!("launching {:#?}", launchable);
            if let Err(e) = launchable.execute(None) {
                warn!("Failed to launch {:?}", &launchable);
                warn!("Error: {:?}", e);
                eprintln!("{e}");
            }
        }
        Ok(None) => {}
        Err(e) => {
            // this usually happens when the passed path isn't of a directory
            warn!("Error: {}", e);
            eprintln!("{e}");
        }
    };
    log_mem(Level::Info);
    info!("bye");
}
