use immt_ontology::{content::declarations::symbols::ArgSpec, narration::{exercises::CognitiveDimension, paragraphs::ParagraphKind}, uris::{DocumentElementURI, DocumentURI, ModuleURI, NarrativeURI, SymbolURI}};
use immt_utils::vecmap::{VecMap, VecSet};
use crate::{components::omdoc::Spec, SHTMLString, SHTMLStringMath};

use super::{content::{ExtensionSpec, ModuleSpec, MorphismSpec, StructureSpec, SymbolSpec}, AnySpec};
use leptos::prelude::*;
use immt_web_utils::components::{Block,Collapsible,Header,HeaderLeft,HeaderRight};
use thaw::{Text,TextTag};

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub struct DocumentSpec {
  pub uri:DocumentURI,
  pub title:Option<String>,
  pub uses:VecSet<ModuleURI>,
  pub children:Vec<DocumentElementSpec>
}
impl super::Spec for DocumentSpec {
    fn into_view(self) -> impl IntoView {
        view!{<Block show_separator=false>
          <HeaderLeft slot>{super::uses("Uses",self.uses.0)}</HeaderLeft>
          {self.children.into_iter().map(super::Spec::into_view).collect_view()}
        </Block>}
    }
}
impl From<DocumentSpec> for AnySpec {
  #[inline]
  fn from(value: DocumentSpec) -> Self {
    Self::Document(value)
  }
}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub struct SectionSpec {
  pub title:Option<String>,
  pub uri:DocumentElementURI,
  pub uses:VecSet<ModuleURI>,
  pub children:Vec<DocumentElementSpec>
}
impl super::Spec for SectionSpec {
    fn into_view(self) -> impl IntoView {
      if let Some(title) = self.title {
        view!{
          <Block>
            <Header slot><b style="font-size:larger"><SHTMLString html=title/></b></Header>
            <HeaderLeft slot>{super::uses("Uses",self.uses.0)}</HeaderLeft>
            {self.children.into_iter().map(super::Spec::into_view).collect_view()}
          </Block>
        }.into_any()
      } else {
        self.children.into_iter().map(super::Spec::into_view)
          .collect_view().into_any()
      }
    }
}
impl From<SectionSpec> for AnySpec {
  #[inline]
  fn from(value: SectionSpec) -> Self {
    Self::Section(value)
  }
}
impl From<SectionSpec> for DocumentElementSpec {
  #[inline]
  fn from(value: SectionSpec) -> Self {
    Self::Section(value)
  }
}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub struct VariableSpec {
  pub uri:DocumentElementURI,
  pub arity:ArgSpec,
  pub macro_name:Option<String>,
  pub tp_html:Option<String>,
  pub df_html:Option<String>,
  pub is_seq:bool
}
impl super::Spec for VariableSpec {
    fn into_view(self) -> impl IntoView {
        let VariableSpec {uri,df_html,tp_html,arity,is_seq,macro_name} = self;
        //let show_separator = !notations.is_empty();
        let varstr = if is_seq {"Sequence Variable "} else {"Variable "};
        view!{
            <Block show_separator=false>
                <Header slot><span>
                    <b>{varstr}<span class="shtml-var-comp">{uri.name().last_name().to_string()}</span></b>
                    {macro_name.map(|name| view!(<span>" ("<Text tag=TextTag::Code>"\\"{name}</Text>")"</span>))}
                </span></Header>
                <HeaderLeft slot><span>{tp_html.map(|html| view! {
                    "Type: "<SHTMLStringMath html/>
                })}</span></HeaderLeft>
                <HeaderRight slot><span style="white-space:nowrap;">{df_html.map(|html| view! {
                    "Definiens: "<SHTMLStringMath html/>
                })}</span></HeaderRight>
                "(TODO: Notation?)"
            </Block>
        }
    }
}
impl From<VariableSpec> for AnySpec {
  #[inline]
  fn from(value: VariableSpec) -> Self {
    Self::Variable(value)
  }
}
impl From<VariableSpec> for DocumentElementSpec {
  #[inline]
  fn from(value: VariableSpec) -> Self {
    Self::Variable(value)
  }
}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub struct ParagraphSpec {
  pub uri:DocumentElementURI,
  pub kind:ParagraphKind,
  pub inline:bool,
  pub uses:VecSet<ModuleURI>,
  pub fors: VecMap<SymbolURI,Option<String>>,
  pub title:Option<String>,
  pub children:Vec<DocumentElementSpec>,
  pub definition_like:bool
}
impl super::Spec for ParagraphSpec {
    fn into_view(self) -> impl IntoView {
        let Self{uri,kind,uses,fors,title,children,definition_like,..} = self;
        let title = title.unwrap_or_else(
          || uri.name().last_name().to_string()
        );
        view!{
          <Block>
            <Header slot><b>
              {super::doc_elem_name(uri,Some(kind.as_display_str()),title)}
            </b></Header>
            <HeaderLeft slot>{super::uses("Uses",uses.0)}</HeaderLeft>
            <HeaderRight slot>{super::comma_sep(
              if definition_like {"Defines"} else {"Concerns"},
              fors.into_iter().map(|(k,html)| view!{
                {super::symbol_name(&k,k.name().last_name().as_ref())}
                {html.map(|html| view!{" as "<SHTMLStringMath html/>})}
              })
            )}</HeaderRight>
            {children.into_iter().map(super::Spec::into_view).collect_view()}
          </Block>
        }
    }
}
impl From<ParagraphSpec> for AnySpec {
  #[inline]
  fn from(value: ParagraphSpec) -> Self {
    Self::Paragraph(value)
  }
}
impl From<ParagraphSpec> for DocumentElementSpec {
  #[inline]
  fn from(value: ParagraphSpec) -> Self {
    Self::Paragraph(value)
  }
}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub struct ExerciseSpec {
  pub uri:DocumentElementURI,
  pub sub_exercise:bool,
  pub autogradable:bool,
  pub points:Option<f32>,
  pub num_solutions:u8,
  pub num_hints:u8,
  pub num_notes:u8,
  pub num_grading_notes:u8,
  pub title:Option<String>,
  pub preconditions:Vec<(CognitiveDimension,SymbolURI)>,
  pub objectives:Vec<(CognitiveDimension,SymbolURI)>,
  pub uses:VecSet<ModuleURI>,
  pub children:Vec<DocumentElementSpec>
}
impl super::Spec for ExerciseSpec {
    fn into_view(self) -> impl IntoView {
        let Self { uri, title, uses, preconditions, objectives, children,..} = self;
        let title = title.unwrap_or_else(
          || uri.name().last_name().to_string()
        );
        view!{
          <Block>
            <Header slot><b>
              {super::doc_elem_name(uri,Some("Exercise"),title)}
            </b></Header>
            <HeaderLeft slot>{super::uses("Uses",uses.0)}</HeaderLeft>
            <HeaderRight slot>{super::comma_sep(
              "Objectives",
              objectives.into_iter().map(|(dim,sym)| view!{
                {super::symbol_name(&sym,sym.name().last_name().as_ref())}
                " ("{dim.to_string()}")"
              })
            )}</HeaderRight>
            {children.into_iter().map(super::Spec::into_view).collect_view()}
          </Block>
        }
    }
}
impl From<ExerciseSpec> for AnySpec {
  #[inline]
  fn from(value: ExerciseSpec) -> Self {
    Self::Exercise(value)
  }
}
impl From<ExerciseSpec> for DocumentElementSpec {
  #[inline]
  fn from(value: ExerciseSpec) -> Self {
    Self::Exercise(value)
  }
}

#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
pub enum DocumentElementSpec {
  Section(SectionSpec),
  Module(ModuleSpec<Self>),
  Morphism(MorphismSpec<Self>),
  Structure(StructureSpec<Self>),
  Extension(ExtensionSpec<Self>),
  DocumentReference {
    uri:DocumentURI,
    title:Option<String>
  },
  Variable(VariableSpec),
  Paragraph(ParagraphSpec),
  Exercise(ExerciseSpec),
  TopTerm(DocumentElementURI,String),
  SymbolDeclaration(either::Either<SymbolURI,SymbolSpec>),
}
impl super::sealed::Sealed for DocumentElementSpec {}
impl super::SpecDecl for DocumentElementSpec {}
impl super::Spec for DocumentElementSpec {
  fn into_view(self) -> impl IntoView {
      match self {
        Self::Section(s) => s.into_view().into_any(),
        Self::Module(m) => m.into_view().into_any(),
        Self::Morphism(m) => m.into_view().into_any(),
        Self::Structure(s) => s.into_view().into_any(),
        Self::Extension(e) => e.into_view().into_any(),
        Self::DocumentReference { uri, title } => doc_ref(uri,title).into_any(),
        Self::Variable(v) => v.into_view().into_any(),
        Self::Paragraph(p) => p.into_view().into_any(),
        Self::Exercise(e) => e.into_view().into_any(),
        Self::TopTerm(uri,html) => view! {
          <Block show_separator=false>
            <Header slot><span><b>"Term "</b><SHTMLStringMath html/></span></Header>
            ""
          </Block>
        }.into_any(),
        Self::SymbolDeclaration(either::Right(s)) => s.into_view().into_any(),
        Self::SymbolDeclaration(either::Left(u)) => 
          view!{<div style="color:red;">"Symbol "{u.to_string()}" not found"</div>}.into_any(),
      }
  }
}

pub(crate) fn doc_ref(uri:DocumentURI,title:Option<String>) -> impl IntoView {
  let name = title.unwrap_or_else(|| uri.name().last_name().to_string());
  let uricl = uri.clone();
  view!{//<Block>
    <Collapsible lazy=true>
      <Header slot><b>"Document "{super::doc_name(&uri, name)}</b></Header>
      <div style="padding-left:15px;">{
        let uri = uricl.clone();
        let r = Resource::new(|| (),move |()| crate::config::server_config.omdoc(NarrativeURI::Document(uri.clone()).into()));
        view!{
          <Suspense fallback=|| view!(<immt_web_utils::components::Spinner/>)>{move || {
            if let Some(Ok((_,omdoc))) = r.get() {
              let AnySpec::Document(omdoc) = omdoc else {unreachable!()};
              Some(omdoc.into_view())
            } else {None}
          }}</Suspense>
        }
      }</div>
    </Collapsible>
    }//</Block>}
}

impl From<DocumentElementSpec> for AnySpec {
  fn from(value: DocumentElementSpec) -> Self {
    match value {
      DocumentElementSpec::Section(s) => Self::Section(s),
      DocumentElementSpec::Module(m) => Self::DocModule(m),
      DocumentElementSpec::Morphism(m) => Self::DocMorphism(m),
      DocumentElementSpec::Structure(s) => Self::DocStructure(s),
      DocumentElementSpec::Extension(e) => Self::DocExtension(e),
      DocumentElementSpec::DocumentReference { uri, title } => Self::DocReference { uri, title },
      DocumentElementSpec::SymbolDeclaration(either::Right(s)) => Self::SymbolDeclaration(s),
      DocumentElementSpec::Variable(v) => Self::Variable(v),
      DocumentElementSpec::Paragraph(p) => Self::Paragraph(p),
      DocumentElementSpec::Exercise(e) => Self::Exercise(e),
      DocumentElementSpec::TopTerm(uri,s) => Self::Term(uri,s),
      DocumentElementSpec::SymbolDeclaration(either::Left(u)) => Self::Other(u.to_string())
    }
  }
}


#[cfg(feature="ssr")]
mod froms {
  use immt_ontology::{content::{declarations::{structures::Extension, OpenDeclaration}, ContentReference, ModuleLike}, narration::{documents::Document, exercises::Exercise, paragraphs::LogicalParagraph, sections::Section, variables::Variable, DocumentElement, NarrationTrait}, uris::{DocumentURI, ModuleURI, SymbolURI}, Checked, DocumentRange};
  use immt_system::backend::{Backend, StringPresenter};
use immt_utils::{vecmap::VecSet, CSS};
  use crate::components::omdoc::content::{ExtensionSpec, ModuleSpec, MorphismSpec, StructureSpec, SymbolSpec};
  use super::{DocumentElementSpec, DocumentSpec, ExerciseSpec, ParagraphSpec, SectionSpec, VariableSpec};

  impl SectionSpec {
    pub fn from_section<B:Backend>(
      Section{title,children,uri,..}:&Section<Checked>,
      presenter:&mut StringPresenter<'_,B>,
      css:&mut VecSet<CSS>
    ) -> Self {
      let mut uses = VecSet::new();
      let mut imports = VecSet::new();
      let title = title.and_then(|r| if let Some((c,s)) = presenter.backend().get_html_fragment(uri.document(),r) {
        if s.trim().is_empty() { None } else {
          for c in c { css.insert(c)}
          Some(s)
        }
      } else {None});
      let children = DocumentElementSpec::do_children(presenter,children,&mut uses,&mut imports,css);
      Self { title, uri:uri.clone(), uses, children }
    }
  }

  impl ParagraphSpec {
    pub fn from_paragraph<B:Backend>(
      LogicalParagraph{uri,kind,inline,fors,title,children,styles,..}:&LogicalParagraph<Checked>,
      presenter:&mut StringPresenter<'_,B>,
      css:&mut VecSet<CSS>,
    ) -> Self {
      let definition_like = kind.is_definition_like(styles);
      let mut uses = VecSet::new();
      let mut imports = VecSet::new();
      let title = title.and_then(|r| if let Some((c,s)) = presenter.backend().get_html_fragment(uri.document(),r) {
        if s.trim().is_empty() { None } else {
          for c in c { css.insert(c)}
          Some(s)
        }
      } else {None});
      let children = DocumentElementSpec::do_children(presenter,children,&mut uses,&mut imports,css);
      Self {
        kind:*kind,inline:*inline,fors:fors.0.iter().map(|(k,v)| (k.clone(),v.as_ref().and_then(|t| presenter.present(t).ok()))).collect(),
        title, uri:uri.clone(), uses, children,
        definition_like
      }
    }
  }

  impl ExerciseSpec {
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_exercise<B:Backend>(
      Exercise{uri,sub_exercise,autogradable,points,solutions,hints,notes,grading_notes,title,preconditions,objectives,children,..}:&Exercise<Checked>,
      presenter:&mut StringPresenter<'_,B>,
      css:&mut VecSet<CSS>
    ) -> Self {
      let mut uses = VecSet::new();
      let mut imports = VecSet::new();
      let title = title.and_then(|r| if let Some((c,s)) = presenter.backend().get_html_fragment(uri.document(), r) {
        if s.trim().is_empty() { None } else {
          for c in c { css.insert(c)}
          Some(s)
        }
      } else {None});
      let children = DocumentElementSpec::do_children(presenter,children,&mut uses,&mut imports,css);
      Self {
        sub_exercise:*sub_exercise,autogradable:*autogradable,points:*points,
        num_solutions:solutions.len() as _,
        num_hints:hints.len() as _,num_notes:notes.len() as _,num_grading_notes:grading_notes.len() as _,
        preconditions:preconditions.to_vec(),
        objectives:objectives.to_vec(),
        title, uri:uri.clone(), uses, children 
      }
    }
  }

  impl VariableSpec {
    pub fn from_variable<B:Backend>(
      Variable{uri,arity,macroname,tp,df,is_seq,..}:&Variable,
      presenter:&mut StringPresenter<'_,B>,
    ) -> Self {
      Self {
        uri:uri.clone(),
        arity:arity.clone(),
        macro_name:macroname.as_ref().map(ToString::to_string),
        tp_html:tp.as_ref().and_then(|t| presenter.present(t).ok()), // TODO
        df_html:df.as_ref().and_then(|t| presenter.present(t).ok()), // TODO
        is_seq:*is_seq,
       }
    }
  }

  impl DocumentElementSpec {
    pub fn from_element<B:Backend>(
        e:&DocumentElement<Checked>,
        presenter:&mut StringPresenter<'_,B>,
        css:&mut VecSet<CSS>,
    ) -> Option<Self> { match e {
      DocumentElement::Section(s) => {
        Some(SectionSpec::from_section(s,presenter,css).into())
      }
      DocumentElement::Paragraph(p) => {
        Some(ParagraphSpec::from_paragraph(p,presenter,css).into())
      }
      DocumentElement::Exercise(p) => {
        Some(ExerciseSpec::from_exercise(p,presenter,css).into())
      }
      _ => None
    }}


    fn do_children<B:Backend>(
      presenter:&mut StringPresenter<'_,B>,
      children:&[DocumentElement<Checked>],
      uses:&mut VecSet<ModuleURI>,
      imports:&mut VecSet<ModuleURI>,
      css:&mut VecSet<CSS>
    ) -> Vec<Self> {
      let mut ret = Vec::new();
      for c in children {match c {
        DocumentElement::Section(s) => {
          ret.push(SectionSpec::from_section(s,presenter,css).into());
        }
        DocumentElement::Paragraph(p) => {
          ret.push(ParagraphSpec::from_paragraph(p,presenter,css).into());
        }
        DocumentElement::Exercise(p) => {
          ret.push(ExerciseSpec::from_exercise(p,presenter,css).into());
        }
        DocumentElement::Module {module,children,..} => {
          let uri = module.id().into_owned();
          let (metatheory,signature) = if let Some(ModuleLike::Module(m)) = module.get() {
            (m.meta().map(|c| c.id().into_owned()),m.signature().map(|c| c.id().into_owned()))
          } else { (None,None) };
          let mut uses = VecSet::new();
          let mut imports = VecSet::new();
          let children = Self::do_children(presenter,children,&mut uses,&mut imports,css);
          ret.push(Self::Module(ModuleSpec { uri, imports, uses, metatheory, signature, children }));
        }
        DocumentElement::Morphism{morphism,children,..} => {
          let uri = morphism.id().into_owned();
          let (total,target) = morphism.get().map_or((false,None),|m|
            (m.as_ref().total,Some(m.as_ref().domain.id().into_owned()))
          );
          let mut uses = VecSet::new();
          let mut imports = VecSet::new();
          let children = Self::do_children(presenter,children,&mut uses,&mut imports,css);
          ret.push(Self::Morphism(MorphismSpec { uri, total, target, uses, children }));
        }
        DocumentElement::MathStructure{structure,children,..} => {
          let uri = structure.id().into_owned();
          let macroname = structure.get().and_then(|s| s.as_ref().macroname.as_ref().map(ToString::to_string));
          let extensions = super::super::froms::get_extensions(presenter.backend(),&uri).map(|e| 
            (
              e.as_ref().uri.clone(),
              e.as_ref().elements.iter().filter_map(|e|
                if let OpenDeclaration::Symbol(s) = e {
                  Some(SymbolSpec::from_symbol(s,presenter))
                } else { None }
              ).collect()
            )
          ).collect();
          let mut uses = VecSet::new();
          let mut imports = VecSet::new();
          let children = Self::do_children(presenter,children,&mut uses,&mut imports,css);
          ret.push(Self::Structure(StructureSpec { uri, macro_name: macroname, uses, extends: imports, children,extensions }));
        }
        DocumentElement::Extension{extension,target,children,..} => {
          let target = target.id().into_owned();
          let uri = extension.id().into_owned();
          let mut uses = VecSet::new();
          let mut imports = VecSet::new();
          let children = Self::do_children(presenter,children,&mut uses,&mut imports,css);
          ret.push(Self::Extension(ExtensionSpec { uri,target, uses, children }));
        }
        DocumentElement::DocumentReference { target,.. } => {
          let title = target.get().and_then(|d| d.title().map(ToString::to_string));
          let uri = target.id().into_owned();
          ret.push(Self::DocumentReference { uri, title });
        }
        DocumentElement::SymbolDeclaration(s) => {
          ret.push(Self::SymbolDeclaration(s.get().map_or_else(|| either::Left(s.id().into_owned()),|s| either::Right(SymbolSpec::from_symbol(s.as_ref(),presenter)))));
        }
        DocumentElement::Variable(v) => {
          ret.push(VariableSpec::from_variable(v,presenter).into());
        }
        DocumentElement::UseModule(m) => {
          uses.insert(m.id().into_owned());
        }
        DocumentElement::ImportModule(m) => {
          imports.insert(m.id().into_owned());
        }
        DocumentElement::TopTerm{term, uri, ..} => {
          ret.push(Self::TopTerm(uri.clone(),presenter.present(term).unwrap_or_else(|e| format!("<mtext>term presenting failed: {e:?}</mtext>"))))
        }
        DocumentElement::Definiendum{..} |
        DocumentElement::SymbolReference {..} |
        DocumentElement::VariableReference {..} |
        DocumentElement::Notation{..} |
        DocumentElement::VariableNotation {..} |
        DocumentElement::SetSectionLevel(_) => ()
      }}
      ret
    }
  }

  impl DocumentSpec {
    pub fn from_document<B:Backend>(
        d:&Document,
        presenter:&mut StringPresenter<'_,B>,
        css:&mut VecSet<CSS>,
    ) -> Self {
      let uri = d.uri().clone();
      let title = d.title().map(ToString::to_string);
      let mut uses = VecSet::new();
      let mut imports = VecSet::new();
      let children = DocumentElementSpec::do_children(presenter,d.children(), &mut uses, &mut imports,css);
      Self { uri, title, uses, children }
    }
  }
}