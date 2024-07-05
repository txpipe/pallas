# Pallas Math

Crate with all the mathematics functions to support Cardano protocol:

- [] lncf - Approximate `ln(1+x)` for `x in 0..infinty`.
- [] cf - Compute continued fraction using max steps or bounded list of a/b factors.
- [] bound - Simple way to find integer powers that bound x.
- [] contract - Bisect bounds to find the smallest integer power such that `factor^n<=x<factor^(n+1)`.
- [] find_e - find n with `e^n<=x<e^(n+1)`.
- [] ln - Compute natural logarithm via continued fraction, first splitting integral part and then using continued fractions approximation for `ln(1+x)`.
- [] taylor_exp - Compute `exp(x)` using Taylor expansion.
- [] taylor_exp_cmp - Efficient way to compare the result of the Taylor expansion of the exponential function to a threshold value.
- ...
- ...