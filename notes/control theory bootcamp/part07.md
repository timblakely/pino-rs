[Part
7](https://www.youtube.com/watch?v=tnsWsMwYbEU&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=7):
Controllability and discrete-time impulse response

Review: controllability matrid $Ç=\begin{bmatrix}BABA^2BA^3B...A^{n-1}B\end{bmatrix}$

Continuous space: $\dot{x}=Ax+Bu$

### Discrete time

- $x_{k+1}=\tilde{A}x_k+\tilde{B}u_k$
  - **IMPORTANT**: $\tilde{A}$ and $\tilde{B}$ are _not the same_ as $A$ and $B$ in continuous time
  - Going back to [Part 3](part03.md), relationship between continuous and discrete is $\tilde{A}=e^{A\Delta t}$
- Investigating "impulse response": kick the system in $u$ and measure what happens to $x$ over time
  - Apply impulse at $t_0$, and nothing after
    - $u_0=1, u_1=0, u_2=0\dots u_m=0$
  - Solve for $x_t$, assuming initial condition $x_0=0$:
    - $x_1=Ax_0 + Bu_0 = A\cdot0+B\cdot1=B$
    - $x_2=Ax_1+Bu_1=A\cdot B$
    - $x_3=Ax_2+Bu_2=A^2\cdot B$
    - ...
    - $x_m=Ax_{m-1}+Bu_{m-1}=A^{m-1}\cdot B$
  - The input "rings" through the system and touches every state in $x\isin\R^n$
    - If the system is hit with an impulse $u$ and it rings through the system and there are some
      dimensions in state space taht _aren't touched by $B$, then some states cannot be controlled
- Assuming all states can be reached by impulse, then system can be controlled by actuator input $B$

