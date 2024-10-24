pub mod archives;
mod cache;
mod docfile;
pub mod rdf;

use archives::{manager::ArchiveManager, source_files::FileState, Archive, ArchiveOrGroup, ArchiveTree, LocalArchive};
use cache::BackendCache;
use docfile::PreDocFile;
use immt_ontology::{
    content::{
        checking::ModuleChecker,
        declarations::{Declaration, DeclarationTrait, UncheckedDeclaration},
        modules::Module,
        ContentReference, ModuleLike
    }, languages::Language, narration::{
        checking::DocumentChecker, documents::Document, DocumentElement, UncheckedDocumentElement,
    }, uris::{
        ArchiveId, ArchiveURI, ArchiveURITrait, ContentURITrait, DocumentURI, ModuleURI, NameStep, PathURIRef, PathURITrait, SymbolURI, URIWithLanguage
    }, DocumentRange, LocalBackend
};
use immt_utils::CSS;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use rdf::RDFStore;
use std::{ops::Deref, path::PathBuf};
use crate::formats::SourceFormatId;

#[derive(Clone, Debug)]
pub enum BackendChange {
    NewArchive(ArchiveURI),
    ArchiveUpdate(ArchiveURI),
    ArchiveDeleted(ArchiveURI),
    FileChange {
        archive: ArchiveURI,
        relative_path: String,
        format: SourceFormatId,
        old: Option<FileState>,
        new: FileState,
    },
}

#[derive(Clone,Debug)]
pub enum AnyBackend{
    Global(&'static GlobalBackend)
}

pub trait Backend {
    fn to_any(&self) -> AnyBackend;
    fn get_document(&self, uri: &DocumentURI) -> Option<Document>;
    fn get_module(&self, uri: &ModuleURI) -> Option<ModuleLike>;
    fn get_base_path(&self,id:&ArchiveId) -> Option<PathBuf>;
    fn get_declaration<T: DeclarationTrait>(&self, uri: &SymbolURI) -> Option<ContentReference<T>>
    where
        Self: Sized;
    fn with_archive_or_group<R>(&self,id:&ArchiveId,f:impl FnOnce(Option<&ArchiveOrGroup>) -> R) -> R
    where
        Self: Sized;

    fn with_archive_tree<R>(&self,f:impl FnOnce(&ArchiveTree) -> R) -> R where Self:Sized;

    fn submit_triples(&self,in_doc:&DocumentURI,rel_path:&str,iter:impl Iterator<Item=immt_ontology::rdf::Triple>)
        where Self:Sized;
    
    fn with_archive<R>(&self, id: &ArchiveId, f: impl FnOnce(Option<&Archive>) -> R) -> R
    where Self:Sized;
    
    fn with_local_archive<R>(
        &self,
        id: &ArchiveId,
        f: impl FnOnce(Option<&LocalArchive>) -> R,
    ) -> R where Self:Sized;
}

impl Backend for AnyBackend {
    #[inline]
    fn to_any(&self) -> AnyBackend {
        self.clone()
    }
    #[inline]
    fn submit_triples(&self,in_doc:&DocumentURI,rel_path:&str,iter:impl Iterator<Item=immt_ontology::rdf::Triple>) {
        match self {
            Self::Global(b) => b.submit_triples(in_doc,rel_path,iter),
        }
    }

    #[inline]
    fn get_document(&self, uri: &DocumentURI) -> Option<Document> {
        match self {
            Self::Global(b) => b.get_document(uri),
        }
    }

    fn with_archive_tree<R>(&self,f:impl FnOnce(&ArchiveTree) -> R) -> R where Self:Sized {
        match self {
            Self::Global(b) => b.with_archive_tree(f),
        }
    }

    #[inline]
    fn get_module(&self, uri: &ModuleURI) -> Option<ModuleLike> {
        match self {
            Self::Global(b) => b.get_module(uri),
        }
    }

    #[inline]
    fn get_base_path(&self,id:&ArchiveId) -> Option<PathBuf> {
        match self {
            Self::Global(b) => b.get_base_path(id),
        }
    }

    #[inline]
    fn get_declaration<T: DeclarationTrait>(&self, uri: &SymbolURI) -> Option<ContentReference<T>>
    where Self: Sized {
        match self {
            Self::Global(b) => b.get_declaration(uri),
        }
    }

    #[inline]
    fn with_archive_or_group<R>(&self,id:&ArchiveId,f:impl FnOnce(Option<&ArchiveOrGroup>) -> R) -> R
    where Self: Sized {
        match self {
            Self::Global(b) => b.with_archive_or_group(id,f),
        }
    }
    
    #[inline]
    fn with_archive<R>(&self, id: &ArchiveId, f: impl FnOnce(Option<&Archive>) -> R) -> R
    where Self:Sized {
        match self {
            Self::Global(b) => b.with_archive(id, f),
        }
    }
    
    #[inline]
    fn with_local_archive<R>(
        &self,
        id: &ArchiveId,
        f: impl FnOnce(Option<&LocalArchive>) -> R,
    ) -> R where Self:Sized {
        match self {
            Self::Global(b) => b.with_local_archive(id, f),
        }
    }
}

#[derive(Debug)]
pub struct GlobalBackend {
    archives: ArchiveManager,
    cache: RwLock<cache::BackendCache>,
    triple_store: RDFStore,
}

lazy_static! {
    static ref GLOBAL: GlobalBackend = GlobalBackend {
        archives: ArchiveManager::default(),
        cache: RwLock::new(cache::BackendCache::default()),
        triple_store: RDFStore::default()
    };
}

impl GlobalBackend {
    #[inline]
    #[must_use]
    pub fn get() -> &'static Self
    where
        Self: Sized,
    {
        &GLOBAL
    }

    pub fn get_html_body(&self,
        d:&DocumentURI,full:bool
    ) -> Option<(Vec<CSS>,String)> {
        self.manager().with_archive(d.archive_id(), |a|
            a.and_then(|a| a.load_html_body(d.path(), d.name().first_name(), d.language(),full))
        )
    }

    #[cfg(feature="tokio")]
    pub async fn get_html_body_async(&self,
        d:&DocumentURI,full:bool
    ) -> Option<(Vec<CSS>,String)> {
        let f = self.manager().with_archive(d.archive_id(), move |a|
            a.map(move |a| a.load_html_body_async(d.path(), d.name().first_name(), d.language(),full))
        )??;
        f.await
    }

    pub fn get_html_fragment(&self,
        d:&DocumentURI,range:DocumentRange
    ) -> Option<(Vec<CSS>,String)> {
        self.manager().with_archive(d.archive_id(), |a|
            a.and_then(|a| a.load_html_fragment(d.path(), d.name().first_name(), d.language(),range))
        )
    }

    #[cfg(feature="tokio")]
    pub async fn get_html_fragment_async(&self,
        d:&DocumentURI,range:DocumentRange
    ) -> Option<(Vec<CSS>,String)> {
        let f = self.manager().with_archive(d.archive_id(), move |a|
            a.map(move |a| a.load_html_fragment_async(d.path(), d.name().first_name(), d.language(),range))
        )??;
        f.await
    }

    #[inline]
    pub const fn manager(&self) -> &ArchiveManager {&self.archives}

    #[inline]
    pub const fn triple_store(&self) -> &RDFStore { &self.triple_store }

    #[inline]
    pub fn all_archives(&self) -> impl Deref<Target = [Archive]> + '_ {
        self.archives.all_archives()
    }

    #[cfg(feature = "tokio")]
    #[allow(clippy::similar_names)]
    #[allow(clippy::significant_drop_tightening)]
    pub async fn get_document_async(&self, uri: &DocumentURI) -> Option<Document> {
        {
            let lock = self.cache.read();
            if let Some(doc) = lock.has_document(uri) {
                return Some(doc.clone());
            }
        }
        let uri = uri.clone();
        tokio::runtime::Handle::current()
            .spawn_blocking(move || {
                let slf = Self::get();
                let mut cache = slf.cache.write();
                let mut flattener = Flattener(&mut cache, &slf.archives);
                flattener.load_document(uri.as_path(), uri.language(), uri.name().first_name())
            })
            .await
            .ok()
            .flatten()
    }

    #[cfg(feature = "tokio")]
    #[allow(clippy::similar_names)]
    #[allow(clippy::significant_drop_tightening)]
    pub async fn get_module_async(&self, uri: &ModuleURI) -> Option<ModuleLike> {
        {
            let lock = self.cache.read();
            if uri.name().is_simple() {
                if let Some(m) = lock.has_module(uri) {
                    return Some(ModuleLike::Module(m.clone()));
                }
            } else {
                let top_uri = !uri.clone();
                if let Some(m) = lock.has_module(&top_uri) {
                    return ModuleLike::in_module(m, uri.name());
                }
            }
        }

        let top = !uri.clone();
        let m = tokio::runtime::Handle::current()
            .spawn_blocking(move || {
                let slf = Self::get();
                let mut cache = slf.cache.write();
                let mut flattener = Flattener(&mut cache, &slf.archives);
                flattener.load_module(top.as_path(), top.language(), top.name().first_name())
            })
            .await
            .ok()??;
        ModuleLike::in_module(&m, uri.name())
    }
}

impl Backend for &'static GlobalBackend {
    #[inline]
    fn to_any(&self) -> AnyBackend {
        AnyBackend::Global(self)
    }

    #[inline]
    fn submit_triples(&self,in_doc:&DocumentURI,rel_path:&str,iter:impl Iterator<Item=immt_ontology::rdf::Triple>) {
        GlobalBackend::submit_triples(self,in_doc,rel_path,iter);
    }

    #[inline]
    fn with_archive_tree<R>(&self,f:impl FnOnce(&ArchiveTree) -> R) -> R where Self:Sized {
        GlobalBackend::with_archive_tree(self, f)
    }

    #[inline]
    fn with_archive<R>(&self, id: &ArchiveId, f: impl FnOnce(Option<&Archive>) -> R) -> R
    {
        GlobalBackend::with_archive(self, id,f)
    }

    #[inline]
    fn with_local_archive<R>(
        &self,
        id: &ArchiveId,
        f: impl FnOnce(Option<&LocalArchive>) -> R,
    ) -> R {
        GlobalBackend::with_local_archive(self, id,f)
    }
    #[inline]
    fn with_archive_or_group<R>(&self,id:&ArchiveId,f:impl FnOnce(Option<&ArchiveOrGroup>) -> R) -> R {
        GlobalBackend::with_archive_or_group(self, id,f)
    }
    #[inline]
    fn get_document(&self, uri: &DocumentURI) -> Option<Document> {
        GlobalBackend::get_document(self, uri)
    }
    #[inline]
    fn get_module(&self, uri: &ModuleURI) -> Option<ModuleLike> {
        GlobalBackend::get_module(self, uri)
    }
    #[inline]
    fn get_base_path(&self,id:&ArchiveId) -> Option<PathBuf> {
        GlobalBackend::get_base_path(self, id)
    }
    #[inline]
    fn get_declaration<T: DeclarationTrait>(&self, uri: &SymbolURI) -> Option<ContentReference<T>> {
        GlobalBackend::get_declaration(self, uri)
    }
}

impl Backend for GlobalBackend {
    #[inline]
    fn to_any(&self) -> AnyBackend {
        AnyBackend::Global(Self::get())
    }

    fn submit_triples(&self,in_doc:&DocumentURI,rel_path:&str,iter:impl Iterator<Item=immt_ontology::rdf::Triple>) {
        //use immt_ontology::rdf::{Triple,Quad,GraphName};
        self.manager().with_archive(in_doc.archive_id(), |a| {
            if let Some(a) = a {
                a.submit_triples(in_doc,rel_path,self.triple_store(),iter);
            }
        });
        /*
        let iri = in_doc.to_iri();
        self.triple_store.add_quads(iter.map(
            |Triple {subject,predicate,object}|
            Quad {subject,predicate,object,graph_name:GraphName::NamedNode(iri.clone())}
        ));*/
    }

    #[inline]
    fn with_archive_tree<R>(&self,f:impl FnOnce(&ArchiveTree) -> R) -> R {
        self.archives.with_tree(f)
    }

    #[inline]
    fn with_archive<R>(&self, id: &ArchiveId, f: impl FnOnce(Option<&Archive>) -> R) -> R
    {
        let archives = &*self.all_archives();
        f(archives.iter().find(|a| a.uri().archive_id() == id))
    }

    #[allow(unreachable_patterns)]
    fn with_local_archive<R>(
        &self,
        id: &ArchiveId,
        f: impl FnOnce(Option<&LocalArchive>) -> R,
    ) -> R
    {
        self.with_archive(id, |a| {
            f(a.and_then(|a| match a {
                Archive::Local(a) => Some(a),
                _ => None,
            }))
        })
    }
    fn with_archive_or_group<R>(&self,id:&ArchiveId,f:impl FnOnce(Option<&ArchiveOrGroup>) -> R) -> R {
        self.with_archive_tree(|t| f(t.find(id)))
    }
    fn get_base_path(&self,id:&ArchiveId) -> Option<PathBuf> {
        self.with_local_archive(id, |a| a.map(|a| a.path().to_path_buf()))
    }

    #[allow(clippy::significant_drop_tightening)]
    fn get_document(&self, uri: &DocumentURI) -> Option<Document> {
        {
            let lock = self.cache.read();
            if let Some(doc) = lock.has_document(uri) {
                return Some(doc.clone());
            }
        }
        let mut cache = self.cache.write();
        let mut flattener = Flattener(&mut cache, &self.archives);
        flattener.load_document(uri.as_path(), uri.language(), uri.name().first_name())
    }

    #[allow(clippy::significant_drop_tightening)]
    fn get_module(&self, uri: &ModuleURI) -> Option<ModuleLike> {
        {
            let lock = self.cache.read();
            if uri.name().is_simple() {
                if let Some(m) = lock.has_module(uri) {
                    return Some(ModuleLike::Module(m.clone()));
                }
            } else {
                let top_uri = !uri.clone();
                if let Some(m) = lock.has_module(&top_uri) {
                    return ModuleLike::in_module(m, uri.name());
                }
            }
        }
        let m = {
            let mut cache = self.cache.write();
            let mut flattener = Flattener(&mut cache, &self.archives);
            flattener.load_module(uri.as_path(), uri.language(), uri.name().first_name())?
        };
        // TODO: this unnecessarily clones
        ModuleLike::in_module(&m, uri.name())
    }

    fn get_declaration<T: DeclarationTrait>(&self, uri: &SymbolURI) -> Option<ContentReference<T>> {
        let m = self.get_module(uri.module())?;
        // TODO this unnecessarily clones
        ContentReference::new(&m, uri.name())
    }
}

struct Flattener<'a>(&'a mut BackendCache, &'a ArchiveManager);
impl Flattener<'_> {
    fn load_document(
        &mut self,
        path: PathURIRef,
        language: Language,
        name: &NameStep,
    ) -> Option<Document> {
        //println!("Document {path}&d={name}&l={language}");
        let pre = self.1.load_document(path, language, name)?;
        let doc_file = PreDocFile::resolve(pre,self);
        let doc = doc_file.clone();
        self.0.insert_document(doc_file);
        Some(doc)
    }
    fn load_module(
        &mut self,
        path: PathURIRef,
        language: Language,
        name: &NameStep,
    ) -> Option<Module> {
        //println!("Module {path}&m={name}&l={language}");
        let pre = self.1.load_module(path, language, name)?;
        let module = pre.check(self);
        self.0.insert_module(module.clone());
        Some(module)
    }
}

impl LocalBackend for Flattener<'_> {
    #[allow(clippy::option_if_let_else)]
    fn get_document(&mut self, uri: &DocumentURI) -> Option<Document> {
        if let Some(doc) = self.0.has_document(uri) {
            Some(doc.clone())
        } else {
            self.load_document(uri.as_path(), uri.language(), uri.name().first_name())
        }
    }

    fn get_module(&mut self, uri: &ModuleURI) -> Option<ModuleLike> {
        if uri.name().is_simple() {
            if let Some(m) = self.0.has_module(uri) {
                return Some(ModuleLike::Module(m.clone()));
            }
        } else {
            let top_uri = !uri.clone();
            if let Some(m) = self.0.has_module(&top_uri) {
                return ModuleLike::in_module(m, uri.name());
            }
        }
        let m = self.load_module(uri.as_path(), uri.language(), uri.name().first_name())?;
        // TODO this unnecessarily clones
        ModuleLike::in_module(&m, uri.name())
    }

    fn get_declaration<T: DeclarationTrait>(
        &mut self,
        uri: &SymbolURI,
    ) -> Option<immt_ontology::content::ContentReference<T>> {
        let m = self.get_module(uri.module())?;
        // TODO this unnecessarily clones
        ContentReference::new(&m, uri.name())
    }
}

impl DocumentChecker for Flattener<'_> {
    #[inline]
    fn open(&mut self, _elem: &mut UncheckedDocumentElement) {}
    #[inline]
    fn close(&mut self, _elem: &mut DocumentElement) {}
}

impl ModuleChecker for Flattener<'_> {
    #[inline]
    fn open(&mut self, _elem: &mut UncheckedDeclaration) {}
    #[inline]
    fn close(&mut self, _elem: &mut Declaration) {}
}
