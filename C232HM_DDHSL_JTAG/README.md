# C232HM_DDHSL_JTAG

如果使用 `c232hm-ddhsl-jtag` 作为 JTAG 接口，则需要连接如下信号:  
```
 C232HM		JTAG/Other
 Num	Color	Func
 1		Red		VCC		Optionally, can power the board if it is not using its own power supply.
 2		Orange	TCK
 3		Yellow  TDI
 4		Green	TDO
 5		Brown   TMS
 6		Grey	GPIOL0 -> as TRST
 10  	Black	Connect to ground
```
其中, `Grey` pin 原本只是GPIO功能, 这里需要将它配置成`TRST`信号, 用于复位芯片, 否则与`JTAG`的握手会失败.  
具体配置方式可以参照cfg文件.  

# 命令  
## 启动`openocd`  
```bash
#sudo openocd -f ./c232hm-ddhsl-jtag.cfg -f PATH_TO_RPI3.cfg 
sudo openocd -f ./c232hm-ddhsl-jtag.cfg -f /usr/share/openocd/scripts/board/rpi3.cfg
# sudo openocd -f ./c232hm-ddhsl-jtag.cfg -f /opt/homebrew/share/openocd/scripts/board/rpi3.cfg

# or
sudo openocd
```

## 连接  
### `gdb`连接  
```bash
gdb-multiarch
target remote :3333
```
### `lldb`连接  
```bash
lldb
gdb-remote localhost:3333
```


