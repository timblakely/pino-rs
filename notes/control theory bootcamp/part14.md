[Part
14](https://www.youtube.com/watch?v=1_UobILf3cc&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=14) -
Linear Quadratic Regulator

Again, more Matlab

[Last time](part13.md) we explored how poles (eigenvalues) affected control. But how do you choose the _best_ poles?

- Use Linear Quadratric Regulator - LQR
  - Idea 1: you can cook up some kind of cost function on how aggressively you want each state to be controlled
    - Invertex pendulum example: you'd much rather penalize $\dot{\theta}$ than, say, $x$ errors
  - Idea 2: you can cook up some kind of cost function on how expensive it is to actuate $u$
    - Example: energy is cheap and abundant, so allow for large $u$; you're on battery or can't actuate very hard with a tiny motor, so ensure small $u$ (usually)
- LQR optimizes a cost function that is the combination of the above ideas
  -
