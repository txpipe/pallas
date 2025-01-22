use std::{borrow::Cow, ops::Deref};

use pallas_primitives::conway::{self, GovActionId};

use crate::{MultiEraGovAction, MultiEraProposal};

impl<'b> MultiEraProposal<'b> {
    pub fn from_conway(x: &'b conway::ProposalProcedure) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(x)))
    }

    pub fn as_conway(&self) -> Option<&conway::ProposalProcedure> {
        match self {
            MultiEraProposal::Conway(x) => Some(x.deref()),
        }
    }

    pub fn deposit(&self) -> u64 {
        match self {
            MultiEraProposal::Conway(x) => x.deposit,
        }
    }

    pub fn reward_account(&self) -> &[u8] {
        match self {
            MultiEraProposal::Conway(x) => x.reward_account.as_ref(),
        }
    }

    pub fn gov_action(&self) -> MultiEraGovAction {
        match self {
            MultiEraProposal::Conway(x) => {
                MultiEraGovAction::Conway(Box::new(Cow::Borrowed(&x.gov_action)))
            }
        }
    }

    pub fn anchor(&self) -> &conway::Anchor {
        match self {
            MultiEraProposal::Conway(x) => &x.anchor,
        }
    }
}

impl<'b> MultiEraGovAction<'b> {
    pub fn from_conway(x: &'b conway::GovAction) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(x)))
    }

    pub fn id(&self) -> Option<GovActionId> {
        match self {
            MultiEraGovAction::Conway(x) => match x.deref().deref().clone() {
                conway::GovAction::ParameterChange(id, ..) => id.into(),
                conway::GovAction::HardForkInitiation(id, ..) => id.into(),
                conway::GovAction::NoConfidence(id) => id.into(),
                conway::GovAction::UpdateCommittee(id, ..) => id.into(),
                conway::GovAction::NewConstitution(id, ..) => id.into(),
                _ => None,
            },
        }
    }
}
