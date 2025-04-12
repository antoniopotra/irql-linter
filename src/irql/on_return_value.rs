use super::IrqlValue;
use crate::error::Error;
use crate::poly_display::PolyDisplay;
use rustc_middle::ty::Instance;
use rustc_middle::ty::PseudoCanonicalInput;

memoize!(
    #[instrument(skip(_cx), fields(poly_instance = %PolyDisplay(&poly_instance)), ret)]
    pub fn instance_on_return_value<'tcx>(
        _cx: &AnalysisCtxt<'tcx>,
        poly_instance: PseudoCanonicalInput<'tcx, Instance<'tcx>>,
    ) -> Result<IrqlValue, Error> {
        Err(Error::TooGeneric)
    }
);
