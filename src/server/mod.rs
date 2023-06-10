//! Code for running the REST API server.

pub mod actix_server;

use clap::Parser;
use hpo::Ontology;

/// Data structure for the web server data.
pub struct WebServerData {
    /// The HPO ontology.
    pub ontology: Ontology,
    /// The database with precomputed Resnik P-values.
    pub db: Option<rocksdb::DBWithThreadMode<rocksdb::MultiThreaded>>,
}

/// Command line arguments for `server pheno` sub command.
#[derive(Parser, Debug)]
#[command(author, version, about = "Run viguno REST API server", long_about = None)]
pub struct Args {
    /// Path to the directory with the HPO files.
    #[arg(long, required = true)]
    pub path_hpo_dir: String,

    /// Whether to suppress printing hints.
    #[arg(long, default_value_t = false)]
    pub suppress_hints: bool,

    /// IP to listen on.
    #[arg(long, default_value = "127.0.0.1")]
    pub listen_host: String,
    /// Port to listen on.
    #[arg(long, default_value_t = 8080)]
    pub listen_port: u16,
}

/// Print some hints via `tracing::info!`.
pub fn print_hints(args: &Args) {
    tracing::info!(
        "Launching server main on http://{}:{} ...",
        args.listen_host.as_str(),
        args.listen_port
    );

    // Short-circuit if no hints are to be
    if args.suppress_hints {
        return;
    }

    // The endpoint `/hpo/genes` provides information related to genes by symbol.
    tracing::info!(
        "  try: http://{}:{}/hpo/genes?gene_symbol=TGDS",
        args.listen_host.as_str(),
        args.listen_port
    );
    // Also, you can query `/hpo/genes` by NCBI gene ID and return the HPO terms of the gene.
    tracing::info!(
        "  try: http://{}:{}/hpo/genes?gene_id=23483&hpo_terms=true",
        args.listen_host.as_str(),
        args.listen_port
    );
    // The `/hpo/omims` term provides information on OMIM terms and can include HPO terms for
    // the disease.
    tracing::info!(
        "  try: http://{}:{}/hpo/omims?omim_id=616145&hpo_terms=true",
        args.listen_host.as_str(),
        args.listen_port
    );
    // The `/hpo/terms` endpoint allows to query by HPO term ID and optionally return a list of
    // genes that are linked to the term.
    tracing::info!(
        "  try: http://{}:{}/hpo/terms?term_id=HP:0000023&genes=true",
        args.listen_host.as_str(),
        args.listen_port
    );
    // We can use `/hpo/sim/term-term` to compute similarity between two HPO term sets `lhs`
    // and `rhs` using a similarity metric.
    tracing::info!(
        "  try: http://{}:{}/hpo/sim/term-term?lhs=HP:0001166,HP:0040069&rhs=HP:0005918,\
        HP:0004188",
        args.listen_host.as_str(),
        args.listen_port
    );
    // The endpoint `/hpo/sim/term-gene` allows to compute the same for a list of `terms` and
    // `gene_symbols`.
    tracing::info!(
        "  try: http://{}:{}/hpo/sim/term-gene?terms=HP:0001166,HP:0000098&gene_symbols=FBN1,TGDS,TTN",
        args.listen_host.as_str(),
        args.listen_port
    );
}

/// Main entry point for `run-server` sub command.
///
/// # Errors
///
/// In the case that there is an error running the server.
pub fn run(args_common: &crate::common::Args, args: &Args) -> Result<(), anyhow::Error> {
    tracing::info!("args_common = {:?}", &args_common);
    tracing::info!("args = {:?}", &args);

    if let Some(level) = args_common.verbose.log_level() {
        match level {
            log::Level::Trace | log::Level::Debug => {
                std::env::set_var("RUST_LOG", "debug");
                env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
            }
            _ => (),
        }
    }

    // Load data that we need for running the server.
    tracing::info!("Loading HPO...");
    let before_loading = std::time::Instant::now();
    let ontology = Ontology::from_standard(&args.path_hpo_dir)?;
    tracing::info!("...done loading HPO in {:?}", before_loading.elapsed());
    tracing::info!("Opening RocksDB for reading...");
    let before_rocksdb = std::time::Instant::now();
    let db = rocksdb::DB::open_cf_for_read_only(
        &rocksdb::Options::default(),
        format!(
            "{}/{}",
            &args.path_hpo_dir, "scores-fun-sim-avg-resnik-gene"
        ),
        ["meta", "scores"],
        true,
    )?;
    tracing::info!("...done opening RocksDB in {:?}", before_rocksdb.elapsed());

    let data = actix_web::web::Data::new(WebServerData {
        ontology,
        db: Some(db),
    });

    // Print the server URL and some hints (the latter: unless suppressed).
    print_hints(args);
    // Launch the Actix web server.
    actix_server::main(args, data)?;

    tracing::info!("All done. Have a nice day!");
    Ok(())
}
