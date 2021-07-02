[Part
13](https://www.youtube.com/watch?v=M_jchYsTZvM&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=13) -
Pole placement for inverted pendulum cart

$\dot{x}=Ax+Bu$ and control part $u=-Kx$, after substitution gives $\dot{x}=(A-BK)x$

Again, another matlab-heavy example. Will just cover relevant commands and explanations here

- From last time, one eigenvalue for inverted pendulum is positive, which means the system is unstable
  - Want to drive the system to stable eigenvalues
  - Using matlab, can pick some arbitrary eigenvalues and ask to solve for $K$, given $A$ and $B$
  - `eigs = [-1.1;-1.2;-1.3;-1.4]`
  - `K=place(A,B,eigs)`, where `eigs` are teh target eigenvalues
    - Cooks up a $K$ such that the resulting eigenvalues of $(A-BK)$ are actually what we asked for
  - Result: $K=\begin{bmatrix}-2.4\\-8.7\\158\\65.5\end{bmatrix}$
    - Double-checking eigenvalues gives correct result: `eig(A-BK)=[-1.4,-1.3,-1.2,-1.1]`
- Next: we use $K$ to stabilize the non-linear system $\frac{d}{dt}\underset{\bar{}}{x}=\underset{\bar{}}{f}(\underset{\bar{}}{x})$
  - During previous `ode45` invocations $u$ was set to $0$
  - Now instead, we leverage $u=-Kx$
  - `u=-K(y-[1;0;pi;0])`
    - Couple of subtleties
      - `y` is actually $x$ here for... reasons
      - $\begin{bmatrix}1\\0\\\pi\\0\end{bmatrix}$ is the _desired_ position, or "reference value" that we want to approach
  - Demo of cart movement, but using initial eigenvalues chosen to be $p=\begin{bmatrix}-.3\\-.4\\-.5\\-.6\end{bmatrix}$ and initial position $x=-3$ (again, position, not state)
    - Cart does move to desired position! However it does so after a long delay and after initially moving off to a more negative state
  - Attempt to make cart movement faster/more aggressive by changing desired eigenvalues: $p=\begin{bmatrix}-1\\-1.1\\-1.2\\-1.3\end{bmatrix}$
    - More "negative" eigenvalues than previous attempt
    - Works well, nice and smooth
  - More aggressive? Sure: $p=\begin{bmatrix}-2\\-2.1\\-2.2\\-2.3\end{bmatrix}$
    - Again, works, but a bit more jerky and somewhat unnatural
  - Even _more_ aggressive? Why not: $p=\begin{bmatrix}-3\\-3.1\\-3.2\\-3.3\end{bmatrix}$
    - Works, but very unnatural and doesn't look like it'd work physically
    - If you plotted the $u$ values along the way, it'd probably be **really** big
- Any further negative movement in eigenvalues results in a broken simulation
  - Likely due to the fact that the controller's assumptions around local linearlity don't hold up in the non-linear system
  - More negative means faster, but not necessarily more stable
- There's a sweet spot of eigenvalues
  - Balance the tradeoff between effort ($u$) and speed
  - Approach is called Linear Quadratic Regulator, or LQR
    - Talked about in [next session](part14.md)
    - Optimal $K$ matrix
