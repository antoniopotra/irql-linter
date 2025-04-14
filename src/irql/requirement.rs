use super::IrqlRequirement;
use crate::error::Error;
use crate::poly_display::PolyDisplay;
use rustc_middle::ty::Instance;
use rustc_middle::ty::PseudoCanonicalInput;

memoize!(
    #[instrument(skip(_cx), fields(poly_instance = %PolyDisplay(&poly_instance)), ret)]
    pub fn instance_requirement<'tcx>(
        _cx: &AnalysisCtxt<'tcx>,
        poly_instance: PseudoCanonicalInput<'tcx, Instance<'tcx>>,
    ) -> Result<IrqlRequirement, Error> {
        Err(Error::TooGeneric)
    }
);
