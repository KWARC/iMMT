use std::sync::Arc;
use oxigraph::model::{GraphName, Quad};
use tracing::{info, info_span, instrument, Span};
use immt_api::archives::ArchiveT;
use crate::backend::archive_manager::ArchiveManager;
use crate::controller::Controller;

pub struct RelationalManager {
    store:Arc<oxigraph::store::Store>
}
impl RelationalManager {
    /*pub fn loader(&mut self) -> oxigraph::store::BulkLoader {
        self.store.bulk_loader()
    }*/
    pub fn size(&self) -> usize {
        self.store.len().unwrap()
    }
    pub fn initialized(&self) -> bool {
        self.size() > 0
    }
    pub fn init(&mut self) {
        let loader = self.store.bulk_loader();
        loader.load_quads(immt_api::ontology::rdf::ulo2::QUADS.iter().copied()).unwrap();
        //relman.loader().load_quads(mgr.get_top().all_archive_triples().map(|t| t.in_graph(GraphName::DefaultGraph))).unwrap();
    }
    pub(crate) fn load_archives(ctrl:Controller) {
        //use tracing::Instrument;
        let span = tracing::Span::current();
        //tokio::spawn(async move {
        std::thread::spawn(move || {
            let _span = span.enter();
            let mgr = ctrl.archives();
            let relman = ctrl.relational_manager();
            relman.load_archives_i(mgr);
        });
        //}.instrument(tracing::Span::current()));
    }
    #[instrument(level="info",name="relational",skip_all)]
    fn load_archives_i(&self,mgr:&ArchiveManager) {
        use tracing_indicatif::span_ext::IndicatifSpanExt;
        use indicatif::ProgressStyle;
        use rayon::iter::ParallelIterator;
        let num = mgr.num_archives();
        let pb = crate::utils::progress::in_progress("📈 Loading relational...")
            .with_length(num as u64).set();

        info!("Loading relational for {num} archives...");

        let old = self.size();
        let store = self.store.clone();
        mgr.par_iter().for_each(move |a|{
            let loader = store.bulk_loader();
            let dir = a.path().join(".out").join("rel.ttl");
            if dir.exists() {
                let iri = a.uri().to_iri();
                let reader = oxigraph::io::RdfParser::from_format(oxigraph::io::RdfFormat::Turtle);
                let mut file = std::fs::File::open(dir).unwrap();
                let mut buf = std::io::BufReader::new(&mut file);
                //std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
                pb.set_message(a.id().as_str());
                let _ = loader.load_quads(
                    reader.parse_read(&mut buf).filter_map(|q| q.ok().map(|q|
                        Quad{subject:q.subject, predicate:q.predicate, object:q.object, graph_name:GraphName::NamedNode(iri.clone())}
                    ))
                );
                pb.tick();
                //Span::current().pb_inc(1);
            }
        });
        info!("Loaded {} relations",self.size()-old);
    }
}
impl Default for RelationalManager {
    fn default() -> Self {
        RelationalManager {
            store:Arc::new(oxigraph::store::Store::new().unwrap())//oxigraph::store::Store::open("foo").unwrap()
        }
    }
}