# Part 25 - [Three equivalent representations of Linear Systems](https://www.youtube.com/watch?v=HasvumXl_vE&list=PLMrJAkhIeNNR20Mz-VpzgfQs5zrYi085m&index=25)

Going to move away from ODE format and move towards transfer functions

In the abstract, a linear dynamic system just takes inputs, does a thing, and produces outputs
- Might not know exactly what's going on under the hood
- Map of something that goes from inputs $u$ to outputs $y$ is a transfer function $G$ (sometimes $P$)

Three equivalent representations for _linear_ systems:
1. State space: $\dot{x}=Ax+Bu$ and $y=Cx$
1. Frequency domain: transfer function $G(s)=C(sI-A)^{-1}B$ where $s$ is Laplacian transform variable
    - Basics of linear system: if $u_1\rightarrow y_1$ and $u_2\rightarrow y_2$, then $u_1 + u_2\rightarrow y_1 + y_2$
1. Time domain: impulse response $y(t)=\int_0^th(t-\tau)y(t)dt$
    - The output of system $y(t)$ is equal to the convolution integral of the impulse response $h(t-\tau)$ and the time-varying input $y(t)$
    - Note that we've seen this before when we were looking at state space system when we were looking at discrete time system ($e^{at}$-based system)
    - Given an impulse in control $Bu$, what's the response e.g. whacking it with a hammer, $h(t-\tau)$ is the response to the impulse
    - If the input (impulse) is changing over time - aka $u(t)$ - all we have to do is convolve it with the impulse response
- Key point: all three are completely equivalent!

What is a transfer function, and what does it mean?
- If you put an input into the system e.g. a sine wave, after transients die off you get the exact same sine wave, but with potential changes of bigger/smaller amplitude and it might have a phase shift
  - $sin(\omega t)$ input, then output is $Asin(\omega t + \phi)$ ($\omega$ being frequency)
- Complex function
  - Remember, $e^{i\omega}$ is like $cos(\omega t) + i\ sin(\omega t)$
  - Plugging in $i\omega$ is like forcing $G$ with a sine wave
- Magnitude: $\left|G(i\omega)\right| = A$
- Phase: $\angle G(i\omega) = \phi$

Idea: if we laplace transform state space, we get transfer function representation in terms of laplace variable $s$
- Essentially if you input different frequencies of sines into your system, how does the output sine look?
  - Bigger? Smaller? Lead? Lag?

Take-away: three equal representations
- If you have an ODE, probably want to use state space to solve LQR or LQE
- If you have data from an experiment, probably want to use impulse response
  - Can use data to back out things like stability properties, eigenvalues, freq response
- Transfer function perspective helps really understand robustness vs performance
