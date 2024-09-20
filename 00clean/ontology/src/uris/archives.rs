use std::convert::Infallible;
use std::fmt::Display;
use std::str::{FromStr, Split};
use const_format::concatcp;
use either::Either;
use lazy_static::lazy_static;
use immt_utils::gc::{TArcInterner, TArcInterned};
use triomphe::Arc;
use parking_lot::Mutex;
use crate::uris::{BaseURI, Name, PathURIRef, PathURITrait, URI, URIRef, URIOrRefTrait, URITrait, URIRefTrait};
use crate::uris::errors::URIParseError;
use crate::uris::macros::debugdisplay;

lazy_static!{
    static ref ARCHIVE_IDS: Arc<Mutex<TArcInterner<str,4,100>>> =
        Arc::new(Mutex::new(TArcInterner::default()));
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArchiveId(TArcInterned<str>);
impl ArchiveId {
    #[must_use]
    pub fn last_name(&self) -> &str {
        let s = self.as_ref();
        s.rsplit_once('/').map_or(s,|(_, s)| s)
    }

    #[must_use]
    pub fn steps(&self) -> std::str::Split<char> {
        self.as_ref().split('/')
    }

    #[must_use]
    pub fn new(s:&str) -> Self { Self(ARCHIVE_IDS.lock().get_or_intern(s)) }
}
impl Ord for ArchiveId {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}
impl PartialOrd for ArchiveId {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl FromStr for ArchiveId {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(ARCHIVE_IDS.lock().get_or_intern(s)))
    }
}
impl AsRef<str> for ArchiveId {
    #[inline]
    fn as_ref(&self) -> &str { self.0.as_ref() }
}

impl Display for ArchiveId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
debugdisplay!(ArchiveId);
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArchiveURI {
    pub(super) base: super::BaseURI,
    pub(super) archive: ArchiveId,
}
impl Display for ArchiveURI {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}?{}={}",self.base,Self::SEPARATOR,self.archive)
    }
}
debugdisplay!(ArchiveURI);

impl ArchiveURI {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(base:BaseURI,archive:ArchiveId) -> Self {
        Self { base,archive }
    }
    pub const SEPARATOR : char = 'a';
    pub(super) fn pre_parse<R>(s:&str,uri_kind:&'static str,f:impl FnOnce(Self,Split<char>) -> Result<R,URIParseError>)
        -> Result<R,URIParseError> {
        let Either::Right((base,mut split)) = BaseURI::pre_parse(s)? else {
            return Err(URIParseError::MissingPartFor {
                uri_kind,
                part: "archive id",
                original:s.to_string()
            });
        };
        let Some(archive) = split.next() else { unreachable!()};
        if !archive.starts_with(concatcp!(ArchiveURI::SEPARATOR,"=")) {
            return Err(URIParseError::MissingPartFor {
                uri_kind,
                part: "archive id",
                original:s.to_string()
            });
        }
        let archive = Self {
            base,archive:ArchiveId::new(&archive[2..])
        };
        f(archive,split)
    }
}
impl FromStr for ArchiveURI {
    type Err = URIParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::pre_parse(s,"archive uri",|a,mut split| {
            if split.next().is_some() {
                return Err(URIParseError::TooManyPartsFor {
                    uri_kind:"archive uri",
                    original:s.to_string()
                });
            }
            Ok(a)
        })
    }
}
impl URIOrRefTrait for ArchiveURI {
    #[inline]
    fn base(&self) -> &BaseURI { &self.base }
    fn as_uri(&self) -> URIRef {
        URIRef::Archive(self.archive_uri())
    }
}
impl URITrait for ArchiveURI {
    type Ref<'a> = ArchiveURIRef<'a>;
}


pub trait ArchiveURITrait: URIOrRefTrait {
    fn archive_uri(&self) -> ArchiveURIRef;

    #[inline]
    fn archive_id(&self) -> &ArchiveId {
        self.archive_uri().archive
    }
}
impl ArchiveURITrait for ArchiveURI {
    #[inline]
    fn archive_uri(&self) -> ArchiveURIRef {
        ArchiveURIRef {
            base:&self.base,
            archive:&self.archive
        }
    }
    #[inline]
    fn archive_id(&self) -> &ArchiveId { &self.archive }
}

impl PathURITrait for ArchiveURI {
    fn as_path(&self) -> PathURIRef {
        PathURIRef {
            archive:self.archive_uri(),
            path:None
        }
    }
    #[inline]
    fn path(&self) -> Option<&Name> { None }
}

#[derive(Clone, Copy,PartialEq, Eq, Hash)]
pub struct ArchiveURIRef<'a> {
    pub(super) base: &'a super::BaseURI,
    pub(super) archive: &'a ArchiveId,
}
impl<'a> ArchiveURIRef<'a> {
    #[inline]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(base:&'a BaseURI,archive:&'a ArchiveId) -> Self {
        Self { base,archive }
    }
}

impl URIOrRefTrait for ArchiveURIRef<'_> {
    #[inline]
    fn base(&self) -> &BaseURI { self.base }
    #[inline]
    fn as_uri(&self) -> URIRef { URIRef::Archive(*self) }
}
impl<'a> URIRefTrait<'a> for ArchiveURIRef<'a> {
    type Owned = ArchiveURI;
    #[inline]
    fn to_owned(self) -> ArchiveURI {
        ArchiveURI {
            base:self.base.clone(),
            archive:self.archive.clone()
        }
    }
}

impl Display for ArchiveURIRef<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}?{}={}",self.base,ArchiveURI::SEPARATOR,self.archive)
    }
}
debugdisplay!(ArchiveURIRef<'_>);
impl PathURITrait for ArchiveURIRef<'_> {
    #[inline]
    fn as_path(&self) -> PathURIRef {
        PathURIRef {
            archive: self.archive_uri(),
            path: None
        }
    }
}

impl ArchiveURITrait for ArchiveURIRef<'_> {
    #[inline]
    fn archive_uri(&self) -> ArchiveURIRef { *self }
}


#[cfg(feature = "serde")]
mod serde_impl {
    use super::{ArchiveURI,ArchiveId};
    use crate::uris::{ArchiveURIRef, serialize};

    serialize!(as+DE ArchiveId);
    serialize!(DE ArchiveURI);
    serialize!(ArchiveURIRef<'_>);
}
