use pallas_primitives::alonzo;

use crate::MultiEraMeta;

impl<'b> MultiEraMeta<'b> {
    pub fn entries(&self) -> &alonzo::Metadata {
        &self.0
    }

    pub fn find(&self, label: alonzo::MetadatumLabel) -> Option<&alonzo::Metadatum> {
        self.entries()
            .iter()
            .find_map(|(key, value)| if key.eq(&label) { Some(value) } else { None })
    }
}
