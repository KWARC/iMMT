use std::fmt::Display;

use crate::{uris::DocumentElementURI, DocumentRange};

use super::{DocumentElement, UncheckedDocumentElement};

#[derive(Debug,Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UncheckedSection {
    pub range: DocumentRange,
    pub uri: DocumentElementURI,
    pub level: SectionLevel,
    pub title: Option<DocumentRange>,
    pub children: Vec<UncheckedDocumentElement>,
}

#[derive(Debug)]
pub struct Section {
    pub range: DocumentRange,
    pub uri: DocumentElementURI,
    pub level: SectionLevel,
    pub title: Option<DocumentRange>,
    pub children: Box<[DocumentElement]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionLevel {
    Part,
    Chapter,
    Section,
    Subsection,
    Subsubsection,
    Paragraph,
    Subparagraph,
}
impl Display for SectionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SectionLevel::*;
        write!(
            f,
            "{}",
            match self {
                Part => "Part",
                Chapter => "Chapter",
                Section => "Section",
                Subsection => "Subsection",
                Subsubsection => "Subsubsection",
                Paragraph => "Paragraph",
                Subparagraph => "Subparagraph",
            }
        )
    }
}
impl TryFrom<u8> for SectionLevel {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, ()> {
        use SectionLevel::*;
        match value {
            0 => Ok(Part),
            1 => Ok(Chapter),
            2 => Ok(Section),
            3 => Ok(Subsection),
            4 => Ok(Subsubsection),
            5 => Ok(Paragraph),
            6 => Ok(Subparagraph),
            _ => Err(()),
        }
    }
}
impl From<SectionLevel> for u8 {
    fn from(s: SectionLevel) -> Self {
        use SectionLevel::*;
        match s {
            Part => 0,
            Chapter => 1,
            Section => 2,
            Subsection => 3,
            Subsubsection => 4,
            Paragraph => 5,
            Subparagraph => 6,
        }
    }
}
