use crate::ServerConfig;

/// Build the multi-threaded Tokio runtime FerroVox runs on.
///
/// Natively scales to all detected CPU cores unless the operator pins a
/// specific worker count in `ferrovox.toml`. This is what gives the
/// server "use all my cores" behavior out of the box on any modern
/// multi-core host.
pub fn build_runtime(cfg: &ServerConfig) -> std::io::Result<tokio::runtime::Runtime> {
    let cores = num_cpus::get();
    let workers = if cfg.performance.worker_threads == 0 {
        cores.max(1)
    } else {
        cfg.performance.worker_threads
    };

    tracing::info!(
        detected_cores = cores,
        worker_threads = workers,
        "building multi-core Tokio runtime"
    );

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .thread_name("ferrovox-worker")
        .enable_all()
        .build()
}
