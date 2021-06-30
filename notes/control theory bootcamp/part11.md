[Part
11](https://www.youtube.com/watch?v=gpIhGAUoeNY&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=11) -
Reachability and controllability via Cayley-Hamilton

$\dot{x}=Ax+Bu$

From last time: $e^{At}=\phi_o(t)I + \phi_1(t)A + \phi_2(t)A^2 + \dots + \phi_{n-1}(t)A^{n-1}$

Remember: _Cayley-Hamilton notation is **finite**_!

### Reachability

- We can show reachability in terms of the control matrix $\mathscr{C}$
- If $\xi\isin\R^n$ is reachable, then $\xi=\int_0^te^{A(t-\tau}Bu(\tau)d\tau$
  - Now substitute in C-H for $e^{A(t-\tau)}$. Going to get long/messy, but bear with it
- $\xi=\int_0^t\begin{pmatrix}\phi_0(t-\tau)IBu(\tau) + \phi_1(t-\tau)ABu(\tau) + \phi_2(t-\tau)A^2Bu(\tau) + \dots + \phi_{n-1}(t-\tau)A^{n-1}Bu(\tau)\end{pmatrix}d\tau$
  - Break up into multiple independant integrals
- $\xi=B\int_0^t\phi_0(t-\tau)u(\tau)d\tau + AB\int_0^t\phi_1(t-\tau)u(\tau)d\tau + A^2B\int_0^t\phi_2(t-\tau)u(\tau)d\tau + \dots + A^{n-1}B\int_0^t\phi_{n-1}(t-\tau)u(\tau)d\tau$
  - Note that these coefficients look almost identical to controllability matrix $\mathscr{C}$, and even match the dimensionality
  - While the $\phi_j$ coefficients may be complex to calculate, $u$ is usually a scalar or small vector
- $\xi=\begin{bmatrix}BABA^2BA^3B...A^{n-1}B\end{bmatrix}
\begin{bmatrix}
\int_0^t\phi_0(t-\tau)u(\tau)d\tau \\
\int_0^t\phi_1(t-\tau)u(\tau)d\tau \\
\int_0^t\phi_2(t-\tau)u(\tau)d\tau \\
\dots \\
\int_0^t\phi_{n-1}(t-\tau)u(\tau)d\tau
\end{bmatrix}
$
  - $\mathscr{C}$ is now explicit on the left
  - Key point: if $\mathscr{C}$ has full rank $n$, by choosing the right $\phi_j$ coefficients we can get _any_ $\xi$ vector
  - If we start at some space in $x\isin\R^n$ and want to get to $\xi$, we _can_ get there if it's controllable
    - Note that there can be infinite number of paths to get there as well, depending on the dimensionality of $u$
    - If $u\isin\R^q$ and $q>1$, then there are multiple $u$ values that can get us to any $\xi$
  - For matrix $\begin{bmatrix}
\int_0^t\phi_0(t-\tau)u(\tau)d\tau \\
\int_0^t\phi_1(t-\tau)u(\tau)d\tau \\
\int_0^t\phi_2(t-\tau)u(\tau)d\tau \\
\dots \\
\int_0^t\phi_{n-1}(t-\tau)u(\tau)d\tau
\end{bmatrix}
$, we need to specify $n\cdot q$ parameters in order to get to specific $\xi$
