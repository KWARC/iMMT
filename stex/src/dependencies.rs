use crate::quickparse::latex::{
    EnvCloseRule, EnvOpenRule, Environment, EnvironmentResult, EnvironmentRule, FromLaTeXToken,
    LaTeXParser, Macro, MacroResult, MacroRule,
};
use immt_api::archives::ArchiveId;
use immt_api::narration::document::Language;
use immt_api::utils::sourcerefs::SourceRange;
use immt_system::utils::parse::{ParseSource, ParseStr};
use std::path::Path;

pub(crate) enum DepToken<'a> {
    ImportModule {
        archive: Option<&'a str>,
        module: &'a str,
    },
    UseModule {
        archive: Option<&'a str>,
        module: &'a str,
    },
    Inputref {
        archive: Option<&'a str>,
        filepath: &'a str,
    },
    Vec(Vec<DepToken<'a>>),
    Signature(Language),
}

pub(crate) enum STeXDependency<'a> {
    ImportModule {
        archive: Option<&'a str>,
        module: &'a str,
    },
    UseModule {
        archive: Option<&'a str>,
        module: &'a str,
    },
    Inputref {
        archive: Option<&'a str>,
        filepath: &'a str,
    },
    Signature(Language),
}

pub(crate) struct DepParser<'a> {
    parser: LaTeXParser<'a, ParseStr<'a, ()>, DepToken<'a>>,
    stack: Vec<std::vec::IntoIter<DepToken<'a>>>,
    curr: Option<std::vec::IntoIter<DepToken<'a>>>,
}

pub(crate) fn get_deps<'a>(source: &'a str, path: &'a Path) -> Vec<STeXDependency<'a>> {
    let parser = LaTeXParser::with_rules(
        ParseStr::new(source),
        Some(path),
        LaTeXParser::default_rules().into_iter().chain(vec![
            ("importmodule", importmodule as MacroRule<'a, _, _>),
            ("usemodule", usemodule as MacroRule<'a, _, _>),
            ("inputref", inputref as MacroRule<'a, _, _>),
        ]),
        LaTeXParser::default_env_rules().into_iter().chain(vec![(
            "smodule",
            (
                smodule_open as EnvOpenRule<'a, _, _>,
                smodule_close as EnvCloseRule<'a, _, _>,
            ),
        )]),
    );
    let mut deps = Vec::new();
    let mut dep_parser = DepParser {
        parser,
        stack: Vec::new(),
        curr: None,
    };
    while let Some(dep) = dep_parser.next() {
        deps.push(dep);
    }
    deps
}

impl<'a> Iterator for DepParser<'a> {
    type Item = STeXDependency<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(curr) = &mut self.curr {
                if let Some(t) = curr.next() {
                    match t {
                        DepToken::ImportModule { archive, module } => {
                            return Some(STeXDependency::ImportModule { archive, module })
                        }
                        DepToken::UseModule { archive, module } => {
                            return Some(STeXDependency::UseModule { archive, module })
                        }
                        DepToken::Signature(lang) => return Some(STeXDependency::Signature(lang)),
                        DepToken::Inputref {
                            archive,
                            filepath: module,
                        } => {
                            return Some(STeXDependency::Inputref {
                                archive,
                                filepath: module,
                            })
                        }
                        DepToken::Vec(v) => {
                            let old = std::mem::replace(&mut self.curr, Some(v.into_iter()));
                            if let Some(old) = old {
                                self.stack.push(old);
                            }
                        }
                    }
                } else {
                    self.curr = self.stack.pop();
                }
            } else {
                match self.parser.next() {
                    Some(DepToken::ImportModule { archive, module }) => {
                        return Some(STeXDependency::ImportModule { archive, module })
                    }
                    Some(DepToken::UseModule { archive, module }) => {
                        return Some(STeXDependency::UseModule { archive, module })
                    }
                    Some(DepToken::Signature(lang)) => {
                        return Some(STeXDependency::Signature(lang))
                    }
                    Some(DepToken::Inputref {
                        archive,
                        filepath: module,
                    }) => {
                        return Some(STeXDependency::Inputref {
                            archive,
                            filepath: module,
                        })
                    }
                    Some(DepToken::Vec(v)) => {
                        self.curr = Some(v.into_iter());
                    }
                    None => return None,
                }
            }
        }
    }
}

impl<'a> FromLaTeXToken<'a, &'a str, ()> for DepToken<'a> {
    fn from_comment(_: SourceRange<()>) -> Option<Self> {
        None
    }
    fn from_group(_: SourceRange<()>, v: Vec<Self>) -> Option<Self> {
        Some(DepToken::Vec(v))
    }
    fn from_math(_: bool, _: SourceRange<()>, v: Vec<Self>) -> Option<Self> {
        Some(DepToken::Vec(v))
    }
    fn from_control_sequence(_: (), _: &'a str) -> Option<Self> {
        None
    }
    fn from_text(_: SourceRange<()>, _: &'a str) -> Option<Self> {
        None
    }
    fn from_macro_application(_: Macro<'a, &'a str, (), Self>) -> Option<Self> {
        None
    }
    fn from_environment(e: Environment<'a, &'a str, (), Self>) -> Option<Self> {
        Some(DepToken::Vec(e.children))
    }
}

pub fn importmodule<'a>(
    mut m: Macro<'a, &'a str, (), DepToken<'a>>,
    p: &mut LaTeXParser<'a, ParseStr<'a, ()>, DepToken<'a>>,
) -> MacroResult<'a, &'a str, (), DepToken<'a>> {
    let archive = p.read_opt_str(&mut m).into_name();
    match p.read_name(&mut m) {
        None => {
            p.tokenizer.problem("Expected { after \\importmodule");
            MacroResult::Simple(m)
        }
        Some(module) => MacroResult::Success(DepToken::ImportModule { archive, module }),
    }
}

pub fn usemodule<'a>(
    mut m: Macro<'a, &'a str, (), DepToken<'a>>,
    p: &mut LaTeXParser<'a, ParseStr<'a, ()>, DepToken<'a>>,
) -> MacroResult<'a, &'a str, (), DepToken<'a>> {
    let archive = p.read_opt_str(&mut m).into_name();
    match p.read_name(&mut m) {
        None => {
            p.tokenizer.problem("Expected { after \\importmodule");
            MacroResult::Simple(m)
        }
        Some(module) => MacroResult::Success(DepToken::UseModule { archive, module }),
    }
}

pub fn inputref<'a>(
    mut m: Macro<'a, &'a str, (), DepToken<'a>>,
    p: &mut LaTeXParser<'a, ParseStr<'a, ()>, DepToken<'a>>,
) -> MacroResult<'a, &'a str, (), DepToken<'a>> {
    if p.tokenizer.reader.starts_with('*') {
        p.tokenizer.reader.pop_head();
    }
    let archive = p.read_opt_str(&mut m).into_name();
    match p.read_name(&mut m) {
        None => {
            p.tokenizer.problem("Expected { after \\inputref");
            MacroResult::Simple(m)
        }
        Some(filepath) => MacroResult::Success(DepToken::Inputref { archive, filepath }),
    }
}

pub fn smodule_open<'a>(
    env: &mut Environment<'a, &'a str, (), DepToken<'a>>,
    p: &mut LaTeXParser<'a, ParseStr<'a, ()>, DepToken<'a>>,
) {
    let opt = p.read_opt_str(&mut env.begin);
    let name = p.read_name(&mut env.begin);
    match opt.as_keyvals().get(&"sig") {
        Some(v) => {
            if let Ok(l) = (*v).try_into() {
                env.children.push(DepToken::Signature(l))
            }
        }
        _ => (),
    }
}
pub fn smodule_close<'a>(
    mut env: Environment<'a, &'a str, (), DepToken<'a>>,
    p: &mut LaTeXParser<'a, ParseStr<'a, ()>, DepToken<'a>>,
) -> EnvironmentResult<'a, &'a str, (), DepToken<'a>> {
    EnvironmentResult::Simple(env)
}