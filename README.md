# USB Blaster for Arduino MKR Vidor 4000

## Regenerate bindings:

```bash
bindgen --use-core --ctypes-prefix "libc" wrapper.h -- ~/.arduino15/packages/arduino/tools/arm-none-eabi-gcc/7-2017q4/arm-none-eabi/include/c++/7.2.1/ -I./USBBlaster/src/ -I./ArduinoCore-samd/variants/mkrvidor4000/ -I./ArduinoCore-samd/cores/arduino/ -I./CMSIS/CMSIS/Include/ -I./ArduinoModule-CMSIS-Atmel/CMSIS-Atmel/CMSIS/Device/ATMEL/samd21/include/ -I/usr/arm-none-eabi/include/ --sysroot ~/.arduino15/packages/arduino/tools/arm-none-eabi-gcc/7-2017q4/arm-none-eabi/ -target thumbv6m-none-eabi -mcpu=cortex-m0plus -mthumb -I ./ArduinoModule-CMSIS-Atmel/CMSIS-Atmel/CMSIS/Device/ATMEL/ -x c++ > src/bindings.rs

 arm-none-eabi-objcopy gccwhee.a libgcc.a -S --add-symbol __gnu_thumb1_case_uqi=0x0000000000005510

 # Grab core.a, variant.cpp.o from Arduino
```

Had to make a bunch of hacks because of obscure errors related to clang and missing deps:

* "anonymous bit field cannot have qualifiers"
* wrapper.h defines fake arduino flags to tell the library you know what you're doing
