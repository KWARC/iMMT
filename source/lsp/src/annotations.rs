use crate::{state::LSPState, IsLSPRange, LSPStore, ProgressCallbackClient};
use async_lsp::lsp_types as lsp;
use immt_ontology::uris::ArchiveURITrait;
use immt_stex::quickparse::stex::{DiagnosticLevel, STeXAnnot, STeXDiagnostic, STeXParseDataI};
use smallvec::SmallVec;
use futures::FutureExt;
use crate::capabilities::STeXSemanticTokens;
use immt_system::backend::{archives::LocalArchive, Backend, GlobalBackend};
use immt_utils::{prelude::TreeChildIter, sourcerefs::{LSPLineCol, SourceRange}};

impl LSPState {
    #[must_use]
    pub fn get_diagnostics(&self,uri:&lsp::Url,progress:Option<ProgressCallbackClient>) -> Option<impl std::future::Future<Output=lsp::DocumentDiagnosticReportResult>> {
        fn default() -> lsp::DocumentDiagnosticReportResult { lsp::DocumentDiagnosticReportResult::Report(
            lsp::DocumentDiagnosticReport::Full(
                lsp::RelatedFullDocumentDiagnosticReport::default()
            )
        )}
        let d = self.get(uri)?;
        let store = LSPStore::<true>::new(self.clone());
        Some(async move { 
            d.with_annots(store,|data| {
                let diags = &data.diagnostics;
                let r = lsp::DocumentDiagnosticReportResult::Report(
                lsp::DocumentDiagnosticReport::Full(
                    lsp::RelatedFullDocumentDiagnosticReport {
                        related_documents:None,
                        full_document_diagnostic_report:lsp::FullDocumentDiagnosticReport {
                            result_id:None,
                            items:diags.iter().map(to_diagnostic).collect()
                        }
                    }
                )
                );
                tracing::trace!("diagnostics: {:?}",r);
                if let Some(p) = progress { p.finish() }
                r
            }).await.unwrap_or_else(default)
        })
    }


    #[must_use]
    pub fn get_symbols(&self,uri:&lsp::Url,progress:Option<ProgressCallbackClient>) -> Option<impl std::future::Future<Output=Option<lsp::DocumentSymbolResponse>>> {

        #[allow(deprecated)]
        fn to_symbols(v:&[STeXAnnot]) -> Vec<lsp::DocumentSymbol> {
            let mut curr = v.iter();
            let mut ret = Vec::new();
            let mut stack = Vec::new();
            //tracing::info!("Annotations: {v:?}");
            loop {
                if let Some(e) = curr.next() { match e {
                    STeXAnnot::Module { uri, full_range, name_range, children,.. } =>{
                        let old = std::mem::replace(&mut curr, children.iter());
                        stack.push((old,lsp::DocumentSymbol {
                            name: uri.to_string(),
                            detail:None,
                            kind:lsp::SymbolKind::MODULE,
                            tags:None,
                            deprecated:None,
                            range:full_range.into_range(),
                            selection_range:name_range.into_range(),
                            children:Some(std::mem::take(&mut ret))
                        }));
                    }
                    STeXAnnot::Symdecl { uri, macroname, main_name_range, name_ranges, full_range, tp, df,.. } => {
                        let sym = lsp::DocumentSymbol {
                            name: uri.to_string(),
                            detail:None,
                            kind:lsp::SymbolKind::OBJECT,
                            tags:None,
                            deprecated:None,
                            range:full_range.into_range(),
                            selection_range:main_name_range.into_range(),
                            children:None
                        };
                        ret.push(sym);
                        /*match (tp,df) {
                        (None,None) =>
                        }*/
                    }
                    STeXAnnot::ImportModule { module, full_range,.. } => {
                        ret.push(lsp::DocumentSymbol {
                            name: format!("import@{}",module.uri),
                            detail:Some(module.uri.to_string()),
                            kind:lsp::SymbolKind::PACKAGE,
                            tags:None,
                            deprecated:None,
                            range:full_range.into_range(),
                            selection_range:full_range.into_range(),
                            children:None
                        });
                    }
                    STeXAnnot::UseModule { module, full_range,.. } => {
                        ret.push(lsp::DocumentSymbol {
                            name: format!("usemodule@{}",module.uri),
                            detail:Some(module.uri.to_string()),
                            kind:lsp::SymbolKind::PACKAGE,
                            tags:None,
                            deprecated:None,
                            range:full_range.into_range(),
                            selection_range:full_range.into_range(),
                            children:None
                        });
                    }
                    STeXAnnot::SetMetatheory { module, full_range,.. } => {
                        ret.push(lsp::DocumentSymbol {
                            name: format!("metatheory@{}",module.uri),
                            detail:Some(module.uri.to_string()),
                            kind:lsp::SymbolKind::NAMESPACE,
                            tags:None,
                            deprecated:None,
                            range:full_range.into_range(),
                            selection_range:full_range.into_range(),
                            children:None
                        });
                    }
                    STeXAnnot::Inputref { archive, filepath, range,.. } => {
                        ret.push(lsp::DocumentSymbol {
                            name: archive.as_ref().map_or_else(
                                    || format!("inputref@{}",filepath.0),
                                    |(a,_)| format!("inputref@[{a}]{}",filepath.0)
                                ),
                            detail:None,
                            kind:lsp::SymbolKind::PACKAGE,
                            tags:None,
                            deprecated:None,
                            range:range.into_range(),
                            selection_range:range.into_range(),
                            children:None
                        });
                    }
                    STeXAnnot::SemanticMacro { .. } => ()
                }} 
                else if let Some((i,mut s)) = stack.pop() {
                    curr = i;
                    std::mem::swap(&mut ret, s.children.as_mut().unwrap_or_else(|| unreachable!()));
                    ret.push(s);
                } else { break }
            }
            //tracing::info!("Returns: {ret:?}");
            ret
        }

        let d = self.get(uri)?;
        let store = LSPStore::new(self.clone());
        Some(d.with_annots(store,|data| {
            let r = lsp::DocumentSymbolResponse::Nested(to_symbols(&data.annotations));
            tracing::trace!("document symbols: {:?}",r);
            if let Some(p) = progress { p.finish() }
            r
        }))
    }

    #[must_use]
    pub fn get_links(&self,uri:&lsp::Url,progress:Option<ProgressCallbackClient>) -> Option<impl std::future::Future<Output=Option<Vec<lsp::DocumentLink>>>> {
        let d = self.get(uri)?;
        let da = d.archive().cloned();
        let store = LSPStore::<true>::new(self.clone());
        Some(d.with_annots(store,move |data| {
            let mut ret = Vec::new();
            for e in <std::slice::Iter<'_,STeXAnnot> as TreeChildIter<STeXAnnot>>::dfs(data.annotations.iter()) {
                match e {
                    STeXAnnot::Inputref { archive, token_range, filepath, range,.. } => {
                        let Some(a) = archive.as_ref().map_or_else(
                            || da.as_ref().map(ArchiveURITrait::archive_id),
                            |(a,_)| Some(a)
                        ) else {continue};
                        let Some(path) = GlobalBackend::get().with_local_archive(a, |a| a.map(LocalArchive::source_dir)) 
                        else { continue };
                        let mut range = *range;
                        range.start = token_range.end;
                        let path = filepath.0.split('/').fold(path, |p,s| p.join(s));
                        let Some(lsp_uri) = lsp::Url::from_file_path(path).ok() else {continue};
                        ret.push(lsp::DocumentLink {
                            range:range.into_range(),
                            target:Some(lsp_uri),
                            tooltip:None,
                            data:None
                        });
                    }
                    STeXAnnot::SetMetatheory { .. } => (),
                    _ => ()
                }
            }
            //tracing::info!("document links: {:?}",ret);
            if let Some(p) = progress { p.finish() }
            ret
        }))
    }

    #[must_use]
    pub fn get_hover(&self,uri:&lsp::Url,position:lsp::Position,progress:Option<ProgressCallbackClient>) -> Option<impl std::future::Future<Output=Option<lsp::Hover>>> {
        let d = self.get(uri)?;
        let store = LSPStore::new(self.clone());
        let pos = LSPLineCol {
            line:position.line,
            col:position.character
        };
        Some(d.with_annots(store,move |data| {
            at_position(data,pos).and_then(|annot| match annot {
                STeXAnnot::SemanticMacro { uri, argnum, token_range, full_range } =>
                    Some(lsp::Hover {
                        range: Some(SourceRange::into_range(*full_range)),
                        contents:lsp::HoverContents::Markup(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: format!("<b>{uri}</b>")
                        })
                    }),
                _ => None
            })
        }).map(|o| o.flatten()))
    }


    #[must_use]
    pub fn get_goto_definition(&self,uri:&lsp::Url,position:lsp::Position,progress:Option<ProgressCallbackClient>) -> Option<impl std::future::Future<Output=Option<lsp::GotoDefinitionResponse>>> {
        let d = self.get(uri)?;
        let store = LSPStore::new(self.clone());
        let pos = LSPLineCol {
            line:position.line,
            col:position.character
        };
        Some(d.with_annots(store,move |data| {
            at_position(data,pos).and_then(|annot| match annot {
                STeXAnnot::ImportModule { module,archive_range,path_range,.. } |
                STeXAnnot::UseModule { module,archive_range,path_range,.. } => {
                    let range = archive_range.map_or(*path_range,|a|
                        SourceRange { start: a.start, end: path_range.end }
                    );
                    if !range.contains(pos) {return None};
                    let Some(p) = module.full_path.as_ref() else {return None};
                    let Ok(uri) = lsp::Url::from_file_path(p) else {return None};
                    Some(lsp::GotoDefinitionResponse::Scalar(lsp::Location {
                        uri,range:lsp::Range::default()
                    }))
                }
                _ => None
            })
        }).map(|o| o.flatten()))
    }


    pub fn get_semantic_tokens(&self,uri:&lsp::Url,progress:Option<ProgressCallbackClient>,range:Option<lsp::Range>) -> Option<impl std::future::Future<Output=Option<lsp::SemanticTokens>>> {
        let range = range.map(SourceRange::from_range);
        let d = self.get(uri)?;
        let store = LSPStore::new(self.clone());
        Some(d.with_annots(store, |data| {
            let mut ret = Vec::new();
            let mut curr = (0u32,0u32);
            macro_rules! make{
                ($rng:expr => $tk:ident) => {
                    let delta_line = $rng.start.line - curr.0;
                    let delta_start = if delta_line == 0 { $rng.start.col - curr.1 } else { $rng.start.col };
                    curr = ($rng.start.line,$rng.start.col);
                    let length = $rng.end.col - $rng.start.col;
                    ret.push(lsp::SemanticToken {
                        delta_line,delta_start,length,
                        token_type:STeXSemanticTokens::$tk,
                        token_modifiers_bitset:0
                    });
                    };
                    ($rng:expr =>> $tk:expr) => {
                    let delta_line = $rng.start.line - curr.0;
                    let delta_start = if delta_line == 0 { $rng.start.col - curr.1 } else { $rng.start.col };
                    curr = ($rng.start.line,$rng.start.col);
                    let length = $rng.end.col - $rng.start.col;
                    ret.push(lsp::SemanticToken {
                        delta_line,delta_start,length,
                        token_type:$tk,
                        token_modifiers_bitset:0
                    });
                }
            }

            for e in <std::slice::Iter<'_,STeXAnnot> as TreeChildIter<STeXAnnot>>::dfs(data.annotations.iter()) {
                match e {
                    STeXAnnot::Symdecl { main_name_range, name_ranges, tp, df, token_range, .. } => {
                        make!(token_range => DECLARATION);
                        make!(main_name_range => NAME);
                        let mut props = SmallVec::<_,3>::new();
                        if let Some((n,t)) = name_ranges {
                            props.push((n,t,Some(STeXSemanticTokens::NAME)));
                        }
                        if let Some((k,v,_)) = tp {
                            props.push((k,v,None));
                        }
                        if let Some((k,v,_)) = df {
                            props.push((k,v,None));
                        }
                        props.sort_by_key(|p| (p.0.start.line,p.0.start.col));
                        for (k,v,t) in props {
                            make!(k => KEYWORD);
                            if let Some(t) = t { make!(v =>> t); }
                        }
                    }
                    STeXAnnot::SemanticMacro{ token_range,..} => {
                        make!(token_range => SYMBOL);
                    }
                    STeXAnnot::Module { uri, name_range, sig, meta_theory, full_range, smodule_range, children } => {
                        make!(smodule_range => DECLARATION);
                        make!(name_range => NAME);
                    }
                    _ => ()
                }
            }

            if let Some(p) = progress { p.finish() }
            lsp::SemanticTokens {
                result_id:None,
                data:ret
            }
        }))
    }

}

fn at_position(data:&STeXParseDataI,position:LSPLineCol) -> Option<&STeXAnnot> {
    let mut ret = None;
    for e in <std::slice::Iter<'_,STeXAnnot> as TreeChildIter<STeXAnnot>>::dfs(data.annotations.iter()) {
        let range = e.range();
        if range.contains(position) {
            ret = Some(e);
        } else if range.start > position {
            if ret.is_some() { break }
        }
    }
    ret
}

#[must_use]
pub fn to_diagnostic(diag:&STeXDiagnostic) -> lsp::Diagnostic {
    lsp::Diagnostic {
        range: diag.range.into_range(),
        severity:Some(match diag.level {
            DiagnosticLevel::Error => lsp::DiagnosticSeverity::ERROR,
            DiagnosticLevel::Info => lsp::DiagnosticSeverity::INFORMATION,
            DiagnosticLevel::Warning => lsp::DiagnosticSeverity::WARNING,
            DiagnosticLevel::Hint => lsp::DiagnosticSeverity::HINT
        }),
        code:None,
        code_description:None,
        source:None,
        message:diag.message.clone(),
        related_information:None,
        tags:None,
        data:None
    }
}