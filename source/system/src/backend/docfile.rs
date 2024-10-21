use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use immt_ontology::{
    narration::documents::{Document, UncheckedDocument},
    uris::DocumentURI,
};

use super::Flattener;

/*
#[derive(Debug)]
pub struct Offsets {
    pub refs_offset: u32,
    pub css_offset: u32,
    pub html_offset: u32,
    pub body_offset: u32,
    pub body_len: u32,
}
*/

#[derive(Debug)]
pub struct HTMLFile {
    path: Box<Path>,
    //offsets: Offsets,
}




#[derive(Debug)]
pub struct DocFile {
    //path: Box<Path>,
    doc: Document,
    //offsets: Offsets,
}

impl DocFile {
    #[inline]
    pub(crate) fn strong_count(&self) -> usize {
        self.doc.strong_count()
    }
    #[inline]
    pub(crate) fn uri(&self) -> &DocumentURI {
        self.doc.uri()
    }
    #[inline]
    pub(crate) const fn doc(&self) -> &Document {
        &self.doc
    }
}

pub struct PreDocFile;

impl PreDocFile {
    pub(crate) fn read_from_file(path: &Path) -> Option<UncheckedDocument> {
        let file = File::open(path).ok()?;
        let file = BufReader::new(file);
        bincode::serde::decode_from_reader(file, bincode::config::standard()).ok()
        //let offsets = Self::read_initials(&mut file)?;
        //let doc = UncheckedDocument::from_byte_stream(&mut file).ok()?;
        //Some(doc)//Some(Self { path, doc, offsets })
    }

    pub(super) fn resolve(doc:UncheckedDocument, flattener: &mut Flattener) -> Document {
        doc.check(flattener)
        /*DocFile {
            //path: self.path,
            //offsets: self.offsets,
            doc,
        }*/
    }
}
/*
pub struct PreDocFile {
    //path: Box<Path>,
    doc: UncheckedDocument,
    //offsets: Offsets,
}

impl PreDocFile {
    /*
    const fn initials_from_buf(buf: [u8; 20]) -> Offsets {
        let refs_start = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let css_start = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let html_start = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let body_start = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let body_len = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        Offsets {
            refs_offset: refs_start + 20,
            css_offset: css_start + 20,
            html_offset: html_start + 20,
            body_offset: body_start + 20,
            body_len,
        }
    }
    */


    fn read_initials(file: &mut BufReader<File>) -> Option<Offsets> {
        let mut buf = [0u8; 20];
        file.read_exact(&mut buf).ok()?;
        Some(Self::initials_from_buf(buf))
    }

    /*
    async fn read_initials_async(file:&mut impl tokio::io::AsyncBufRead,path:Box<Path>) -> Option<PreDocFile> {
      let mut buf = [0u8;20];
      file.read_exact(&mut buf).await.ok()?;
      Some(Self::initials_from_buf(path,buf))
    }

    pub(crate) async fn read_from_file_async(path:Box<Path>) -> Option<(PreDocFile,UncheckedDocument)> {
      use tokio::io::AsyncBufReadExt;
      let file = tokio::fs::File::open(&path).await.ok()?;
      let predoc = Self::read_initials_async(&mut file, path).await?;
      let doc = UncheckedDocument::from_byte_stream_async(&mut file).await.ok()?;
      Some((predoc,doc))
    }
    */
}

*/