## Z80 console Emulator
This is a Z80 emulator that implement a personal console. This is not compatible with any commercial consoles.
### Memory MAP
```
Screen Memory: 8K
E000---FFFF
screen text resolution: 80x25 = 2000 char space
E000--E7D0 <== 8bits ASCII char memory
BackGroundColor: RGB : R=0E7D1 , G=0E7D2 , B=0E7D3
forwardColor: RGB : R=0E7D4 , G=0E7D5 , B=0E7D6
cursorXposition: 0xE7D7
cursorYposition: 0xE7D8
E800--FFFF : Not use


RAM Memory: 56K
0000--DFFF

```