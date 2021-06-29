[Part
10](https://www.youtube.com/watch?v=PrfxmkBsYKE&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=10) -
Cayley-Hamilton Theorem

$\dot{x}=Ax+Bu$

- Gem in terms of linear algebra
- Theorem: (almost) every square matrix $A$ satisfies its own characteristic (eigenvalue) equation
- Eigen decomp: $det(A-\lambda I)=0$
- $\lambda^n+a_{n-1}\lambda^{n-1}+a_{n-2}\lambda^{n-2}+a_{n-3}\lambda^{n-3}+\dots+a_1\lambda + a_0=0$
  - Roots of which are eigenvalues
- Here's where theorem comes in: Plug in $A$ for $\lambda$
  - $A^n+a_{n-1}A^{n-1}+a_{n-2}A^{n-2}+a_{n-3}A^{n-3}+\dots+a_1A + a_0I=0$
  - Rearrange: $A^n=-a_{n-1}A^{n-1}-a_{n-2}A^{n-2}-a_{n-3}A^{n-3}-\dots-a_1A - a_0I$
  - Rephrasing as sum: $A^{\geq n}=\sum\limits_{j=0}^{n-1}\alpha_jA^j$

This has implications with the solution to $\dot{x}=Ax$: $e^{At}$

- $e^{At}$ can be written as $e^{At}=I+At+\frac{A^2t^2}{2}+\frac{A^3t^3}{3}+\dots$
  - This is an **infinite sum**; requires total summation over infinite terms for actual solution
  - However, applying Cayley-Hamilton means that we can represent $e^{At}$ as **finite sum** over $n$ terms
  - $e^{At}=\alpha_0(t)I + \alpha_1(t)A + \alpha_2(t)A^2 + \alpha_3(t)A^3+...+ \alpha_{n-1}(t)A^{n-1}A^2$. Period. _Not_ infinite.
  - This _does_ require time-varying coefficients for the matricies, but that's (apparently?) simpler to handle
- Applying this will allow us to show that controllability $\iff$ reachability
  - Sneak peek: notice how $Ç=\begin{bmatrix}BABA^2BA^3B...A^{n-1}B\end{bmatrix}$ is finite, and up to $A^{n-1}$ as well
