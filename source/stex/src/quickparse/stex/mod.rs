pub mod rules;

use std::path::Path;

use chrono::format::parse;
use immt_ontology::{languages::Language, uris::{ArchiveId, ArchiveURITrait, DocumentURI, ModuleURI, SymbolURI}};
use immt_system::backend::AnyBackend;
use immt_utils::{parsing::ParseStr, prelude::{TreeChild, TreeLike}, sourcerefs::{LSPLineCol, SourceRange}, vecmap::VecSet};
use rules::{ModuleReference, ModuleRules, NotationArgs, STeXModuleStore, STeXParseState, STeXToken, SymbolReference, SymdeclArgs};
use smallvec::SmallVec;

use super::latex::LaTeXParser;

#[derive(Default)]
pub struct STeXParseDataI {
  pub annotations: Vec<STeXAnnot>,
  pub diagnostics: VecSet<STeXDiagnostic>,
  pub modules:SmallVec<(ModuleURI,ModuleRules<LSPLineCol>),1>
}
impl STeXParseDataI {
  #[inline]#[must_use]
  pub fn lock(self) -> STeXParseData {
    immt_utils::triomphe::Arc::new(parking_lot::Mutex::new(self))
  }
  #[inline]
  pub fn replace(self,old:&STeXParseData) {
    *old.lock() = self;
  }
  #[inline]#[must_use]
  pub fn is_empty(&self) -> bool {
    self.annotations.is_empty() && self.diagnostics.is_empty()
  }
}

pub type STeXParseData = immt_utils::triomphe::Arc<parking_lot::Mutex<STeXParseDataI>>;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum STeXAnnot {
  Module {
    uri:ModuleURI,
    name_range:SourceRange<LSPLineCol>,
    sig:Option<(Language,SourceRange<LSPLineCol>)>,
    meta_theory:Option<(ModuleReference,Option<SourceRange<LSPLineCol>>)>,
    full_range: SourceRange<LSPLineCol>,
    smodule_range:SourceRange<LSPLineCol>,
    children:Vec<Self>
  },
  SemanticMacro {
    uri:SymbolReference<LSPLineCol>,
    argnum:u8,
    token_range: SourceRange<LSPLineCol>,
    full_range: SourceRange<LSPLineCol>
  },
  ImportModule {
    archive_range: Option<SourceRange<LSPLineCol>>,
    path_range: SourceRange<LSPLineCol>,
    module: ModuleReference,
    token_range: SourceRange<LSPLineCol>,
    full_range: SourceRange<LSPLineCol>
  },
  UseModule {
    archive_range: Option<SourceRange<LSPLineCol>>,
    path_range: SourceRange<LSPLineCol>,
    module: ModuleReference,
    token_range: SourceRange<LSPLineCol>,
    full_range: SourceRange<LSPLineCol>
  },
  SetMetatheory {
    archive_range: Option<SourceRange<LSPLineCol>>,
    path_range: SourceRange<LSPLineCol>,
    module: ModuleReference,
    token_range: SourceRange<LSPLineCol>,
    full_range: SourceRange<LSPLineCol>
  },
  Inputref {
    archive: Option<(ArchiveId,SourceRange<LSPLineCol>)>,
    filepath: (std::sync::Arc<str>,SourceRange<LSPLineCol>),
    token_range: SourceRange<LSPLineCol>,
    range: SourceRange<LSPLineCol>
  },
  #[allow(clippy::type_complexity)]
  Symdecl {
    uri:SymbolReference<LSPLineCol>,
    macroname:Option<String>,
    main_name_range:SourceRange<LSPLineCol>,
    name_ranges:Option<(SourceRange<LSPLineCol>,SourceRange<LSPLineCol>)>,
    parsed_args:Box<SymdeclArgs<LSPLineCol,Self>>,
    token_range: SourceRange<LSPLineCol>,
    full_range: SourceRange<LSPLineCol>
  },
  #[allow(clippy::type_complexity)]
  Symdef {
    uri:SymbolReference<LSPLineCol>,
    macroname:Option<String>,
    main_name_range:SourceRange<LSPLineCol>,
    name_ranges:Option<(SourceRange<LSPLineCol>,SourceRange<LSPLineCol>)>,
    parsed_args:Box<SymdeclArgs<LSPLineCol,Self>>,
    notation_args:NotationArgs<LSPLineCol,Self>,
    token_range: SourceRange<LSPLineCol>,
    notation:(SourceRange<LSPLineCol>,Vec<Self>),
    full_range: SourceRange<LSPLineCol>
  }
}
impl STeXAnnot {
  fn from_tokens<I:IntoIterator<Item=STeXToken<LSPLineCol>>>(iter: I,mut modules:Option<&mut SmallVec<(ModuleURI,ModuleRules<LSPLineCol>),1>>) -> Vec<Self> {
    let mut v = Vec::new();
    for t in iter {
      match t {
        STeXToken::Module { uri, name_range, sig, meta_theory, full_range, smodule_range, children,rules } => {
          if let Some(ref mut m) = modules { m.push((uri.clone(),rules)) };
          v.push(STeXAnnot::Module { uri, name_range, sig, meta_theory, full_range, smodule_range, children:Self::from_tokens(children,None) });
        }
        STeXToken::SemanticMacro { uri, argnum, token_range, full_range } => 
          v.push(STeXAnnot::SemanticMacro { uri, argnum, token_range, full_range }),
        STeXToken::ImportModule { archive_range, path_range, module, token_range, full_range } => 
          v.push(STeXAnnot::ImportModule { archive_range, path_range, module, token_range, full_range }),
        STeXToken::UseModule { archive_range, path_range, module, token_range, full_range } => 
          v.push(STeXAnnot::UseModule { archive_range, path_range, module, token_range, full_range }),
        STeXToken::SetMetatheory { archive_range, path_range, module, token_range, full_range } => 
          v.push(STeXAnnot::SetMetatheory { archive_range, path_range, module, token_range, full_range }),
        STeXToken::Inputref { archive, filepath, token_range, full_range } => 
          v.push(STeXAnnot::Inputref { archive, filepath, token_range, range:full_range }),
        STeXToken::Symdecl { uri, macroname, main_name_range, name_ranges, token_range, full_range, parsed_args } =>
          v.push(STeXAnnot::Symdecl { uri, macroname, main_name_range, name_ranges, token_range, full_range, 
            parsed_args:Box::new(parsed_args.into_other(|v| Self::from_tokens(v,if let Some(m) = modules.as_mut() { Some(*m) } else { None } )))
          }),
        STeXToken::Symdef { uri, macroname, main_name_range, name_ranges, token_range, full_range, parsed_args, notation_args, notation } =>
        v.push(STeXAnnot::Symdef { uri, macroname, main_name_range, name_ranges, token_range, full_range, 
          parsed_args:Box::new(parsed_args.into_other(|v| Self::from_tokens(v,if let Some(m) = modules.as_mut() { Some(*m) } else { None } ))),
          notation_args:notation_args.into_other(|v| Self::from_tokens(v,if let Some(m) = modules.as_mut() { Some(*m) } else { None } )),
          notation:(notation.0,Self::from_tokens(notation.1,None))
        }),
        STeXToken::Vec(vi) => v.extend(Self::from_tokens(vi,if let Some(m) = modules.as_mut() { Some(*m) } else { None } )),
      }
    }
    v
  }
}

impl STeXAnnot {
  #[must_use]#[inline]
  pub const fn range(&self) -> SourceRange<LSPLineCol> {
    match self {
      Self::Module { full_range, .. } |
      Self::SemanticMacro { full_range, .. } |
      Self::ImportModule { full_range, .. } |
      Self::UseModule { full_range, .. } |
      Self::SetMetatheory { full_range, .. } |
      Self::Symdecl { full_range, .. } |
      Self::Symdef  { full_range, .. } => *full_range,
      Self::Inputref { range, .. } => *range,
    }
  }
}

impl TreeLike for STeXAnnot {
  type Child<'a> = &'a Self;
  type RefIter<'a> = std::slice::Iter<'a, Self>;
  fn children(&self) -> Option<Self::RefIter<'_>> {
    match self {
      Self::Module { children, .. } => Some(children.iter()),
      _ => None
    }
  }
}
impl TreeChild<STeXAnnot> for &STeXAnnot {
  fn children<'a>(&self) -> Option<std::slice::Iter<'a, STeXAnnot>>
      where
          Self: 'a {
    <STeXAnnot as TreeLike>::children(self)
  }
}

#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum DiagnosticLevel {
  Error,Warning,Info,Hint
}

#[derive(PartialEq,Eq)]
pub struct STeXDiagnostic {
  pub level: DiagnosticLevel,
  pub message: String,
  pub range: SourceRange<LSPLineCol>
}

#[must_use]
pub fn quickparse<'a,S:STeXModuleStore>(uri:&'a DocumentURI,source: &'a str,path:&'a Path,backend:&'a AnyBackend,store:S) -> STeXParseDataI {
  let mut diagnostics = VecSet::new();
  let mut modules = SmallVec::new();
  let err = |message,range| diagnostics.insert(STeXDiagnostic {
    level:DiagnosticLevel::Warning,
    message, range
  });
  let mut parser = if S::FULL  { 
    LaTeXParser::with_rules(
      ParseStr::new(source),
      STeXParseState::new(Some(uri.archive_uri()),Some(path),uri,backend,store),
      err,
      LaTeXParser::default_rules().into_iter().chain(
        rules::all_rules()
      ),
      LaTeXParser::default_env_rules().into_iter().chain(
        rules::all_env_rules()
      )
    )
  } else {
    LaTeXParser::with_rules(
      ParseStr::new(source),
      STeXParseState::new(Some(uri.archive_uri()),Some(path),uri,backend,store),
      err,
      LaTeXParser::default_rules().into_iter().chain(
        rules::declarative_rules()
      ),
      LaTeXParser::default_env_rules().into_iter().chain(
        rules::declarative_env_rules()
      )
    )
  };

  let annotations = STeXAnnot::from_tokens(parser, Some(&mut modules));
  STeXParseDataI { annotations, diagnostics, modules }
}