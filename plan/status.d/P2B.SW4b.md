## STATUS

## P2B.SW4b: Fit statistics and Compare models take metafor fits again, 2026-11-26

- SW4's "this works for fitted models" check left out `rma.uni`, so Objects > Model > Fit statistics and Compare models refused a meta-analysis fitted from the Model menu (P6.11c). The two guards now list `rma.uni`; Average marginal effects still do not take it. Found by `cargo test -p rcraft-engine --test meta` (meta_regression_matches_meta_regress_and_has_the_model_menu) after SW4 and P6.11c both landed.
