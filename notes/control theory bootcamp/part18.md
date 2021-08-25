# Part 18 [The Kalman Filter](https://www.youtube.com/watch?v=s_9InuQAx-g&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=18)

Finally ready to build Kalman Filter
- Analog of LQR for estimation
- Optimal, full-state estimator given some knowledge about the types of disturbances $w_d$ and types
  of measurement noise $w_n$ that the system will experience
- Assume distrubance $w_d$ is gaussian, white-noise process with a variance $V_d$ (co-variance)
  - Square matrix of size $n\times n$
- Assume sensor noise $w_n$ is also gaussian, white-noise process with a variance $V_n$ (co-variance)

Consider whether kicks $u$ system will experience are larger or smaller than the measurement noise
$w_n$
- Get to trust one more than another
  - If sensor noise is large, can't trust $y$ very much so it has to rely on its model $\dot{x}$ more
  - If it has the potential to get kicked out from where it should be predicted (by $w_d$), then it
    should trust its measurements more
- Based on the ratio of the variance matrices $V_d$ and $V_n$
- Remember from last time the error measurement is $\mathcal{E}=x-\hat{x}$, and that $\dot{\mathcal{E}}=\left(A-K_fC\right)\mathcal{E}$
  - Using $\dot{\mathcal{E}}$, can make $\hat{x}$ converge arbitrarily quickly (modify rate of
    change of $\dot{\mathcal{E}}$) by choosing the right Kalman Filter gain $K_f$
  - But in real life there's a sweet spot
  - Minimize cost function $J=\mathbb{E}\left(\left(x-\hat{x}\right)^T\left(x-\hat{x}\right)\right)$ ($\mathbb{E}$ is "expectation value")
    - Meaning: pick $K_f$ such that the _expected value_ of error $\left(x-\hat{x}\right)$ is minimized

Doesn't look like it, but $J$ can be written in a form that looks almost identical to LQR
$J=\int_{0}^{\infin}\left(x^TQx + u^TRu\right)dt$
- Can use same lin. alg approach to find $K_f$ as we did with LQR
  - [Algebraic Riccati equation](https://en.wikipedia.org/wiki/Algebraic_Riccati_equation)
  - Order $O\left(n^3\right)$
- Octave: `Kf=lqe(A, C, Vd, Vn))`
  - `lqe` is Linear Quadratic Estimator
