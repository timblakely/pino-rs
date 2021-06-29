[Part
9](https://www.youtube.com/watch?v=PrfxmkBsYKE&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=9) -
Controllability and PBH test

$\dot{x}=Ax+Bu$
General idea: want to know if we can design a control $u=-Kx$

## Popov-Beleritch-Houtus (PBH) test

- $(A, B)$ is controllable $iff$ $rank\begin{bmatrix}(A-\lambda I)B\end{bmatrix}=n$ for
  $\forall\lambda\isin\Complex$ (complex)
  - Space is $n\times n$
- So when does $rank\begin{bmatrix}(A-\lambda I)B\end{bmatrix}$ have rank $n$?
  - When the matrix $(A-\lambda I)$ is rank deficient, i.e. $det(A-\lambda I)=0$
    - In other words the eigenvalue equation
  - Note that due to the eigen decomposition relationship, it's only satisfied by n special eigenvalues!
  - Thus, if $\lambda$ is not an eigenvalue of $A$, $(A-\lambda I)$ has rank $n$
    - For these cases, we don't even need to worry about $B$!

Rules:

1. $rank(A-\lambda I)=n$ for all $\lambda$ except for eigenvalues!
    - Technically need to satisfy $\forall\lambda\isin\Complex$, but really only need to test it at the eigenvalues due to eigen decomp above
    - At most need to test $n$ values of $\lambda$

So we plug in eigenvalue $\lambda$ and the result is rank deficient, but what in what direction?
  - The null space of $(A-\lambda I)$, aka the eigenvalues
  - Thus the only way $(A-\lambda I)$ _is_ rank deficient is in the eigenvector directions
- Thus, for system to be controllable, actuation set of vectors $B$ has to complement the eigenvector direction
  - Has to have at least some component in the direction of the eigenvector direction
  - Allows $B$ to be linearly independent from $(A-\lambda I)$

2. $B$ needs to have some component in each eigenvector direction

  - In order to have $rank\begin{bmatrix}(A-\lambda I)B\end{bmatrix}=n$ for all $\lambda$, $B$ **must** have some component in the corresponding eigenvector if $rank\begin{bmatrix}(A-\lambda I)B\end{bmatrix}<n$

3. (Advanced) If $B$ is a random vector i.e. $B=randn(n,1)$, $(A,B)$ will be controllable with high probability (!)
  - Very unlikely to have zeros in any column, so there's always at least a tiny component in all eigenvector directions

### Steppping back

- Up to now, we've been treating $B$ as a single column vector; one "control knob" $u$
- But $u$ could be multi-dimensional
  - The PBH test can tell you the _minimum_ dimension of $u$
  - $rank\begin{bmatrix}(A-\lambda I)B\end{bmatrix}$ is rank deficient in more than one direction only if it has an eigenvalue with multiplicity > 1
    - "Multiplicity" $\approxeq$ repeated
  - If eigenvalues of A has a multiplicity of 2 (e.g. $1,1$), that means we need at least two dimensions of $u$, and two corresponding columns of $B$
- Gray area: what if two eigenvalues are _really_ close?
  - Not _technically_ degenerate/rank deficient, but can be particularly hard to control with a single knob
  - Solution: add another knob to $u$ to compensate and make the system more controllable

### Take-aways

1. We want $B$ to be able to reach all eigenvectors of $A$
1. We want redundant columns in $B$ to compensate for multiplicity in eigenvalues and/or very close values of $\lambda$
