# Probe grounding

Based on [this
post](https://electronics.stackexchange.com/questions/136123/how-do-you-attach-an-oscilloscope-ground-spring),
an interesting part to potentially build into boards for truly grounded probe testing is
[MMCX](http://www.digikey.com/product-search/en?FV=fff40016%2Cfff8051a&k=MMCX&mnonly=0&newproducts=0&ColumnSort=1000011&page=1&stock=1&pbfree=0&rohs=0&quantity=&ptm=0&fid=0&pageSize=250)
connectors. From there you can get some [premade MMXC->SMA
pigtails](http://www.digikey.com/product-search/en/cable-assemblies/coaxial-cables-rf/1573243?k=MMCX)
(and subsequently convert to [BNC](https://en.wikipedia.org/wiki/BNC_connector)) for pretty cheap. Neat

# Micros

## STM32G474RE

[Datasheet](../third_party/st/documents/stm32g474re.pdf)

[Reference manual](../third_party/st/documents/rm0440-stm32g4-series-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)


## STM32H725VGT6/STM32H735VGT6

[Newark](https://www.newark.com/stmicroelectronics/stm32h735vgt6/mcu-32bit-550mhz-lqfp-100-rohs/dp/89AH1358?st=stm32h7) has the 35 coming in November for $15.79@10+.

# Drivers

[MCF8316A](https://www.ti.com/product/MCF8316A#features) from TI
- Complete FoC (but sensorless) in a tiny package. _Includes FETs!_

# FETs

TI's [CSD88599Q5DC](https://www.ti.com/product/CSD88599Q5DC) are two 60V FETs in a single package.

# Passives

## Ceramics

TDK's C3216X7R1H106K160AE has soft termination and not awful derating
