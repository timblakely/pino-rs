# Part 19 - [Example in Matlab of designing Kalman filter](https://www.youtube.com/watch?v=XBI_hQRqMvM&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=19)

Light notes; matlab episode

![](images/2021-08-24-18-21-05.png)

State is linear in pendulum up+down

State $\underbar{x}=\begin{bmatrix} x \\ \dot{x} \\ \theta \\ \dot{\theta}\end{bmatrix}$

$\dot{x}=Ax+Bu$
- Input $u$ is a force on cart (4x1 column vector)
- Previously had full state; now looking at partially-observed state $y=Cx$

Observability `obsv(A, C)`
- If rank is full (4), can build estimator for $x$ based solely on $y$

Say we could only measure $x$, that means $C=\begin{bmatrix}1&0&0&0\end{bmatrix}$
- Observability matrix $\omicron=\begin{bmatrix}C \\ CA \\ CA^2 \\ CA^3\end{bmatrix}$
- Ends up being $\begin{bmatrix}1 \\ & 1 \\ & -0.2 & 2 \\ & 0.04 & -0.4 & 2\end{bmatrix}$ for his values
  - Lower triangular matrix, so can just spot inspect that it has full rank (nonzero determinant of 4)
- Can just observe $x$ to back out all the state variables $\underbar{x}$

Say we could only measure $\theta$, that means $C=\begin{bmatrix}0&0&1&0\end{bmatrix}$
- Det is 0, as is first full column of $\omicron$
- Since first column is all 0s, means it's "Translationally invariant"
  - System doesn't care where it is horizontally for control

Take-away: If you want to get to horizontal position $x$, you _must_ measure $x$ since it can't be
inferred from any other combination of states.
