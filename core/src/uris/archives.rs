use std::fmt::Display;
use std::ops::Div;
use std::str::FromStr;
use triomphe::Arc;
use crate::uris::base::BaseURI;
use crate::ontology::rdf::terms::NamedNode;
use crate::uris::Name;

lazy_static::lazy_static! {
    static ref IDS:Arc<lasso::ThreadedRodeo<lasso::MiniSpur,rustc_hash::FxBuildHasher>> = Arc::new(lasso::ThreadedRodeo::with_hasher(rustc_hash::FxBuildHasher::default()));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArchiveId(lasso::MiniSpur);

#[cfg(feature = "serde")]
impl serde::Serialize for ArchiveId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(IDS.resolve(&self.0))
    }
}
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ArchiveId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

impl ArchiveId {
    #[inline]
    pub fn as_str(&self) -> &'static str { IDS.resolve(&self.0) }
    #[inline]
    pub fn is_empty(&self) -> bool { self.as_str().is_empty() }
    pub fn last_name(&self) -> &'static str {
        let s = self.as_str();
        s.rsplit_once('/').map(|(_, s)| s).unwrap_or(s)
    }
    #[inline]
    pub fn steps(&self) -> std::str::Split<'static,char> {
        self.as_str().split('/')
    }
    #[inline]
    pub fn new(s: impl AsRef<str>) -> Self { Self(IDS.get_or_intern(s)) }
    pub fn is_meta(&self) -> bool {
        self.last_name().eq_ignore_ascii_case("meta-inf")
    }
}
impl FromStr for ArchiveId {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}
impl<S: AsRef<str>,I:IntoIterator<Item=S>> From<I> for ArchiveId {
    fn from(v: I) -> Self {
        let mut inner = String::new();
        for s in v {
            inner.push_str(s.as_ref());
            inner.push('/');
        }
        inner.pop();
        Self::new(&inner)
    }
}
impl Display for ArchiveId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> Div<&'a ArchiveId> for &'a BaseURI {
    type Output = ArchiveURI;
    fn div(self, rhs: &'a ArchiveId) -> Self::Output {
        ArchiveURI { base: *self, archive: *rhs }
    }
}
impl<S:Into<ArchiveId>> Div<S> for BaseURI {
    type Output = ArchiveURI;
    fn div(self, rhs: S) -> Self::Output {
        ArchiveURI { base: self, archive: rhs.into() }
    }
}
impl<S:Into<ArchiveId>> Div<S> for &BaseURI {
    type Output = ArchiveURI;
    fn div(self, rhs: S) -> Self::Output {
        ArchiveURI { base: *self, archive: rhs.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArchiveURI {
    base: BaseURI,
    archive: ArchiveId,
}
impl FromStr for ArchiveURI {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('?');
        let base = parts.next().ok_or(())?;
        let archive = parts.next().ok_or(())?;
        if !archive.starts_with("a=") {
            return Err(());
        }
        let archive = ArchiveId::new(&archive[2..]);
        Ok(ArchiveURI {
            base: BaseURI::new(base).map_err(|_| ())?,
            archive,
        })
    }

}

#[cfg(feature = "serde")]
impl serde::Serialize for ArchiveURI {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
#[cfg(feature = "serde")]
impl<'d> serde::Deserialize<'d> for ArchiveURI {
    fn deserialize<D: serde::Deserializer<'d>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let mut parts = s.split('?');
        let base = parts.next().ok_or_else(|| serde::de::Error::custom("No base"))?;
        let archive = parts.next().ok_or_else(|| serde::de::Error::custom("No archive"))?;
        if !archive.starts_with("a=") {
            return Err(serde::de::Error::custom("No archive"));
        }
        let archive = ArchiveId::new(&archive[2..]);
        Ok(ArchiveURI {
            base: BaseURI::new(base).map_err(serde::de::Error::custom)?,
            archive,
        })
    }
}

impl ArchiveURI {
    pub fn new(dom: BaseURI, archive: ArchiveId) -> Self {
        Self { base: dom, archive }
    }
    #[inline]
    pub fn base(&self) -> BaseURI {
        self.base
    }
    #[inline]
    pub fn id(&self) -> ArchiveId {
        self.archive
    }
    pub fn to_iri(&self) -> NamedNode {
        NamedNode::new(format!("{}?a={}", self.base(), self.id())).unwrap()
    }
}
impl Display for ArchiveURI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}?a={}", self.base, self.archive)
    }
}
/*
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ArchiveURIRef<'a> {
    dom: BaseURI,
    archive: &'a ArchiveId,
}
impl<'a> ArchiveURIRef<'a> {
    #[inline]
    pub fn base(&self) -> BaseURI {
        self.dom
    }
    #[inline]
    pub fn id(&self) -> &'a ArchiveId {
        self.archive
    }
    pub fn to_owned(&self) -> ArchiveURI {
        ArchiveURI {
            dom: self.dom.clone(),
            archive: self.archive.clone(),
        }
    }
    pub fn to_iri(&self) -> NamedNode {
        NamedNode::new(format!("{}?a={}", self.base(), self.id())).unwrap()
    }
}
impl<'a> Display for ArchiveURIRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}?a={}", self.dom, self.archive)
    }
}

impl PartialEq<ArchiveURI> for ArchiveURIRef<'_> {
    fn eq(&self, other: &ArchiveURI) -> bool {
        self.dom == other.dom && self.archive == &other.archive
    }
}
impl<'a> PartialEq<ArchiveURIRef<'a>> for ArchiveURI {
    fn eq(&self, other: &ArchiveURIRef<'a>) -> bool {
        self.dom == other.dom && &self.archive == other.archive
    }
}

 */

#[cfg(test)]
mod tests {
    use crate::tests::*;
    use crate::uris::base::BaseURI;

    #[rstest]
    fn archive_uris(setup:()) {
        let dom = BaseURI::new("http://mathhub.info/:sTeX").unwrap();
        let id = super::ArchiveId::new("test/general");
        let borrowed = &dom / &id;
        info!("Borrowed: {borrowed}");
        let owned = dom.clone() / borrowed.id().clone();
        info!("Owned: {owned}");
        assert_eq!(borrowed, owned);
        info!("Scheme: {}, host: {}, authority: {}, path: {}",
            dom.scheme(),
            dom.host().unwrap_or(""),
            dom.authority(),
            dom.path()
        );
    }
}
