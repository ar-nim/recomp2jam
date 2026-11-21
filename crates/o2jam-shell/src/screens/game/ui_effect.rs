//! UI-effect ids used by shell handler branches (ticket #721).
//!
//! The 7.55 emitter at VA `0x551870` (file `0x151870`) takes a cache slot
//! below `0x34`; a null sample slot returns failure, and an active sample
//! is stopped before the new one starts. The slot is *not* the decimal
//! sample name: the boot bank table at VA `0x67f728` (file `0x27f728`,
//! stride `0x48`) adds each bank's base to the fetch ordinal written by
//! the entry fetcher (`0x54eca0`). Bank 0 `bgEffect.ojm` has base `0`.
//!
//! Evidence: `/home/arnim/scratch/ui-sfx-static-findings.md`, sections 1–2
//! (emitter dossier `ev_de048ee2…`, bank-name dossier
//! `ev_a9fb8aef…`, and the per-handler dossiers cited per interaction).

/// Highest cache slot the 7.55 emitter accepts (ids `0..0x33`).
pub const UI_EFFECT_SLOT_COUNT: usize = 0x34;

/// One pinned UI-effect cache slot: the id the exe passes to `0x551870`
/// from the handler branch named by the variant. Several branches share
/// a slot; the variant keeps the caller's intent at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEffectId {
    /// Dialog OK / accept: exe `0x4345b0` and `0x434c00`, slot `0x01`.
    DialogAccept,
    /// Planet emblem pick: exe `0x48e316`, slot `0x01`.
    EmblemPick,
    /// Dialog decline / cancel: exe `0x4345fc` and `0x434c4c`, slot `0x02`.
    DialogDecline,
    /// Close with confirm (planet exit disc, exe `0x48e087`), slot `0x05`.
    PlanetExit,
    /// Channel row join (exe `0x47530d`), slot `0x09`.
    ChannelRowJoin,
    /// Option tab switch (exe `0x435b90` family), slot `0x09`.
    OptionTabSwitch,
    /// Sound-tab slider apply (exe `0x43b552`), slot `0x02`.
    SliderApply,
    /// Channel BACK / EXIT (exe `0x48ff45`), slot `0x23`.
    ChannelExit,
    /// MyRoom WEBAVATAR notice (exe `0x4a3ee0`, file `0xa3eb0`,
    /// EV `ev_aa64e525…`), slot `0x0D` = decimal sound 13.
    WebAvatarNotice,
}

impl UiEffectId {
    /// Exe cache slot carried by the emitter call.
    #[must_use]
    pub const fn slot(self) -> u8 {
        match self {
            Self::DialogAccept | Self::EmblemPick => 0x01,
            Self::DialogDecline | Self::SliderApply => 0x02,
            Self::PlanetExit => 0x05,
            Self::ChannelRowJoin | Self::OptionTabSwitch => 0x09,
            Self::ChannelExit => 0x23,
            Self::WebAvatarNotice => 0x0D,
        }
    }

    /// The canonical pinned cue for one emitter slot, if this cluster
    /// names it. Slots outside `0..0x34` are rejected like the exe's
    /// emitter bound.
    #[must_use]
    pub fn from_slot(slot: u8) -> Option<Self> {
        match slot {
            0x01 => Some(Self::DialogAccept),
            0x02 => Some(Self::DialogDecline),
            0x05 => Some(Self::PlanetExit),
            0x09 => Some(Self::ChannelRowJoin),
            0x0D => Some(Self::WebAvatarNotice),
            0x23 => Some(Self::ChannelExit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{UiEffectId, UI_EFFECT_SLOT_COUNT};

    #[test]
    fn pinned_effects_round_trip_through_their_cache_slots() {
        for effect in [
            UiEffectId::DialogAccept,
            UiEffectId::DialogDecline,
            UiEffectId::PlanetExit,
            UiEffectId::ChannelRowJoin,
            UiEffectId::ChannelExit,
            UiEffectId::WebAvatarNotice,
        ] {
            assert_eq!(UiEffectId::from_slot(effect.slot()), Some(effect));
            assert!(usize::from(effect.slot()) < UI_EFFECT_SLOT_COUNT);
        }
        assert_eq!(
            UiEffectId::from_slot(UiEffectId::OptionTabSwitch.slot()),
            Some(UiEffectId::ChannelRowJoin),
            "slot 9 has two named callers; row join is canonical"
        );
        assert_eq!(
            UiEffectId::from_slot(UiEffectId::EmblemPick.slot()),
            Some(UiEffectId::DialogAccept),
            "slot 1 has two named callers; dialog OK is canonical"
        );
    }

    #[test]
    fn slider_apply_and_dialog_decline_share_slot_two() {
        assert_eq!(
            UiEffectId::SliderApply.slot(),
            UiEffectId::DialogDecline.slot()
        );
        assert_eq!(UiEffectId::SliderApply.slot(), 2);
    }

    #[test]
    fn tab_switch_and_channel_row_join_share_slot_nine() {
        assert_eq!(
            UiEffectId::OptionTabSwitch.slot(),
            UiEffectId::ChannelRowJoin.slot()
        );
        assert_eq!(UiEffectId::ChannelRowJoin.slot(), 9);
    }

    #[test]
    fn an_unpinned_or_out_of_range_slot_names_no_effect() {
        assert_eq!(UiEffectId::from_slot(0), None);
        assert_eq!(UiEffectId::from_slot(0x34), None);
        assert_eq!(UiEffectId::from_slot(0xFF), None);
    }
}
