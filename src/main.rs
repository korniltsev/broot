use std::io::Write;
use cli_log::*;


#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
#[export_name = "malloc_conf"]
pub static malloc_conf: &[u8] = b"prof:true,prof_active:true,lg_prof_sample:18\0";

async fn pyroscope_loop() {
    {
        jemalloc_pprof::activate_jemalloc_profiling().await;
    }
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        {
            //todo termination
            let Some(ctl) = jemalloc_pprof::PROF_CTL.as_ref() else {
                continue;
            };
            let mut ctl = ctl.lock().await;
            let Ok(pprof) = ctl.dump_pprof() else {
                continue;
            };
        }
    }
}


fn main() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(pyroscope_loop())
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
