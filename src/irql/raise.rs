use super::IrqlValue;
use crate::error::Error;
use crate::poly_display::PolyDisplay;
use rustc_hir::def_id::CrateNum;
use rustc_middle::ty::Instance;
use rustc_middle::ty::PseudoCanonicalInput;

memoize!(
    #[instrument(skip(_cx), fields(poly_instance = %PolyDisplay(&poly_instance)), ret)]
    pub fn instance_raise<'tcx>(
        _cx: &AnalysisCtxt<'tcx>,
        poly_instance: PseudoCanonicalInput<'tcx, Instance<'tcx>>,
    ) -> Result<IrqlValue, Error> {
        Err(Error::TooGeneric)
    }
);

impl crate::ctxt::PersistentQuery for instance_raise {
    type LocalKey<'tcx> = Instance<'tcx>;

    fn into_crate_and_local(key: Self::Key<'_>) -> (CrateNum, Self::LocalKey<'_>) {
        let instance = key.value;
        (instance.def_id().krate, instance)
    }
}
