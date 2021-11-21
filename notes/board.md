Rev 1.1 changes required:

- [ ] Solid ground plane
- [ ] 1k pullups on SPI lines instead of 10k
- [ ] 4.7uF and 4.7R RC filter on LDO input
- [ ] If room, 47uF electrolytic
- [ ] Change ceramics to C3216X7R1H106K160AE
- [ ] Motor connector using [DF22R-3P-8.92DS](https://www.digikey.com/en/products/detail/hirose-electric-co-ltd/DF22R-3P-7-92DS-05/1025058)
- [ ] Remove under-FET through-holes
- [ ] Add place to clip when measuring w/oscilloscope
- [ ] Vias for sensing all three phases, not just Phase A

Rev 2.0

- [ ] Leverage [ISO7762](https://www.ti.com/lit/ds/symlink/iso7761.pdf?ts=1636337587281&ref_url=https%253A%252F%252Fwww.ti.com%252Fproduct%252FISO7761) for galvanic isolation on SWD and CAN
- [ ]  5V isolation
- [ ]  Power over USB C?
- [ ]  500 uOhm sense resistors
