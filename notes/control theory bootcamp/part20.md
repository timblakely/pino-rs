# Part 20 - [Example in Matlab of designing Kalman filter pt 2](https://www.youtube.com/watch?v=DLytfA10RR8&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=20)

What happens if you don't care about cart's horizontal position, and only want to stabilize angle?
  - Side note, I'll bet this is what Ben was talking about when he was designing his [furuta pendulum](https://build-its-inprogress.blogspot.com/2016/08/desktop-inverted-pendulum-part-2-control.html)
    - Side side note: That post is a gold mine
- There can be a _substate_ within the state $\underbar{x}=\begin{bmatrix} x \\ \dot{x} \\ \theta \\ \dot{\theta}\end{bmatrix}$
  - i.e. if you didn't care about $x$, the substate would be $\underbar{x}=\begin{bmatrix} \dot{x} \\ \theta \\ \dot{\theta}\end{bmatrix}$

Build a "subsystem" of the lower $3\times 3$ $A$ and $B$ matrices, since $C$ only has three columns
- Observing only $\dot{x}$ means $C=\begin{bmatrix}1&0&0\end{bmatrix}$, results in an observability matrix $\omicron$ with $\det\omicron=something\ nonzero$, so observable
- Observing only $\theta$ means $C=\begin{bmatrix}0&1&0\end{bmatrix}$, results in an observability matrix $\omicron$ with $\det\omicron=-0.2$, so observable
- Observing only $\theta$ means $C=\begin{bmatrix}0&0&1\end{bmatrix}$, results in an observability matrix $\omicron$ with $\det\omicron=-0.1$, so observable

Now looking at the Gramian
- $\dot{x}=Ax+Bu$
- $y=Cx + \cancel{Du}$ (matlab needs `D` to be populated)
- Still same $3\times 3$ subsystem, and only measuring $\dot{\theta}$ via  $C=\begin{bmatrix}0&0&1\end{bmatrix}$
- Can build "state space" system via `sys = ss(A,B,C,D)`
- Determinant of Gramian `det(gram(sys, 'o'))`, with `'o'` meaning observability
  - Determinant gives the volume of the observability "ellipsoid"
  - Higher determinant (ellipsoid) is, the higher the signal-to-noise ratio
  - Note that `gram` cannot be used for unstable systems! Must be a stable pole/zero (in this case, the pendulum down)
    - Built on $e^{At}$, and that blows up in unstable systems

- Various determinants:
$$
\begin{array}{c:c}
  measurement\  y & \det w_{\omicron} \\ \hline
  \dot{x} & 50 \\
  \theta & .03 \\
  \dot{\theta} & .03
\end {array}
$$
  - Much larger observability ellipsoid if you measure $\dot{x}$ vs $\theta$ or $\dot{\theta}$
  - Get a lot of "gain" if you measure $\dot{x}$
