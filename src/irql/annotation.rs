use crate::{attribute::Irql, ctxt::AnalysisCtxt};
use rustc_hir::def_id::{CrateNum, DefId, DefIndex};
use rustc_span::sym;

impl AnalysisCtxt<'_> {
    fn irql_annotation_fallback(&self, def_id: DefId) -> Irql {
        Irql::default()
    }

    fn core_out_of_band_irql_annotation(&self, def_id: DefId) -> Irql {
        Irql::default()
    }
}

memoize!(
    pub fn irql_annotation<'tcx>(cx: &AnalysisCtxt<'tcx>, def_id: DefId) -> Irql {
        if cx.crate_name(def_id.krate) == sym::core {
            return cx.core_out_of_band_irql_annotation(def_id);
        }

        let Some(local_def_id) = def_id.as_local() else {
            if let Some(irql) = cx.sql_load::<irql_annotation>(def_id) {
                return irql;
            }
            return cx.irql_annotation_fallback(def_id);
        };

        let hir_id = cx.local_def_id_to_hir_id(local_def_id);
        for attr in cx.klint_attributes(hir_id).iter() {
            if let crate::attribute::KlintAttribute::Irql(irql) = attr {
                return *irql;
            }
        }

        Default::default()
    }
);

impl crate::ctxt::PersistentQuery for irql_annotation {
    type LocalKey<'tcx> = DefIndex;

    fn into_crate_and_local(key: Self::Key<'_>) -> (CrateNum, Self::LocalKey<'_>) {
        (key.krate, key.index)
    }
}

memoize!(
    pub fn drop_irql_annotation<'tcx>(cx: &AnalysisCtxt<'tcx>, def_id: DefId) -> Irql {
        let Some(local_def_id) = def_id.as_local() else {
            if let Some(irql) = cx.sql_load::<drop_irql_annotation>(def_id) {
                return irql;
            }
            return cx.irql_annotation_fallback(def_id);
        };

        let hir_id = cx.local_def_id_to_hir_id(local_def_id);
        for attr in cx.klint_attributes(hir_id).iter() {
            if let crate::attribute::KlintAttribute::DropIrql(irql) = attr {
                return *irql;
            }
        }

        Default::default()
    }
);

impl crate::ctxt::PersistentQuery for drop_irql_annotation {
    type LocalKey<'tcx> = DefIndex;

    fn into_crate_and_local(key: Self::Key<'_>) -> (CrateNum, Self::LocalKey<'_>) {
        (key.krate, key.index)
    }
}
