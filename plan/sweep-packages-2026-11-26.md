# Sweep of the dialogs that needed a package, 2026-11-26

After `sweep-final-2026-11-26.md`, fixest, plm, rstanarm, sampleSelection, lavaan and vars
(and gt, flextable, officer, glmmTMB, metafor, betareg, margins, coefplot, psych) were
installed into RCraft's own library with the user's OK. The catalog sweep was run again on main
(fd77e81) with the user's R 4.6.2, then a targeted run gave each of the 15 dialogs that had
waited for a package sensible choices on a made-up balanced panel (30 units, 8 periods), the
post-fit dialogs on models the Bayesian and Cox dialogs had fitted. The targeted harness is
saved as `sweep-targeted-harness.rs.txt` (append to the harness; never committed as source).

## Catalog sweep, all 137 dialogs

73 ran Ok, 21 the harness could not fill, 43 errors, none waiting for a package. The
errors were judged: the 15 package dialogs failed only because the harness indexes a panel
by continuous columns (no complete units, singletons, an unbalanced panel, a non-binary selection
variable), fits no model for the Bayesian post-fit dialogs, or names no data frame: harness
noise. Hence the targeted run.

## The 15 dialogs

| Dialog | Result |
|---|---|
| Fixed-effects regression (clustered; two-way, conventional) | ran ok |
| Fixed-effects count or binary (Poisson; logit, drops all-0/all-1 units with a note) | ran ok |
| Panel model (fixed effects; random with clustered errors) | ran ok |
| Hausman test | ran ok |
| Dynamic panel model (difference, two-step robust; system, one-step) | ran ok |
| Panel instrumental variables (within; random) | ran ok |
| Panel unit-root test (Levin-Lin-Chu; Im-Pesaran-Shin) | ran ok |
| Two-stage least squares (with and without fixed effects) | ran ok |
| Heckman selection model (ML; two-step) | ran ok |
| Structural equation model (helpers; typed lavaan lines) | ran ok (a negative-variance warning on made-up indicators is lavaan's) |
| Vector autoregression (2 lags, 8-step impulse responses) | ran ok |
| Bayesian regression (Gaussian; logit) | ran ok |
| Posterior predictions (expected; new outcome) | ran ok (adds columns; prints nothing) |
| Trace plots and predictive checks | ran ok |
| Compare Bayesian models (loo) (alone; with a second fit) | ran ok (loo warns when the two fits have different outcomes, as it should) |

Other findings, none fixed: Heckman prints R's one-off "Registered S3 method overwritten by
'car'" message (from loading sampleSelection) at the top of the output, harmless (low).

## Fixed

- F1 (SW18): Survival curves from a Cox model with no Cox model now says "this needs a Cox model
  fitted with coxph" (for survival, cumulative hazard and hazard), not R's "object 'cox' not found".
