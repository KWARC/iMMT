use immt_core::building::formats::SourceFormatId;
use crate::backend::manager::{ArchiveManager};
use crate::building::targets::SourceFormat;
use crate::extensions::{ExtensionId, MMTExtension};
use crate::utils::settings::Settings;

pub trait Controller:Send+Sync {
    fn archives(&self) -> &ArchiveManager;
    fn log_file(&self) -> &std::path::Path;
    fn build_queue(&self) -> &crate::building::buildqueue::BuildQueue;
    fn settings(&self) -> &Settings;
    fn get_format(&self, id:SourceFormatId) -> Option<&SourceFormat>;
    fn get_extension(&self,id:ExtensionId) -> Option<&dyn MMTExtension>;
}