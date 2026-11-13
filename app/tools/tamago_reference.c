// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Opt-in host scheduler for unchanged T7Emu CPU, bus, EEPROM and sound handlers.
#include "t7_internal.h"
#include "t7_sound.h"
extern void host_set_rom(unsigned char*,size_t);
extern void figure_rom_set_active(const unsigned char*,unsigned);
extern unsigned char t7_dbg_p(void);
int main(int argc,char**argv){FILE*f=fopen(argv[1],"rb");fseek(f,0,2);long n=ftell(f);rewind(f);unsigned char*b=malloc(n);fread(b,1,n,f);fclose(f);host_set_rom(b,n);if(argc>3){f=fopen(argv[3],"rb");fseek(f,0,2);n=ftell(f);rewind(f);b=malloc(n);fread(b,1,n,f);fclose(f);figure_rom_set_active(b,n>4194304?4194304:n);t7_figure_set(1);}t7_init();t7_snd_config(4000000,44100);t7_snd_set_mode(1);f=fopen(argv[2],"wb");unsigned long long cyc=0,video=4000000/60,tbh=2000000,key_until=0;unsigned char last_keys=255;for(int tick=1;tick<=3000;tick++){int keys=(tick>=300&&tick<320)||(tick>=520&&tick<540)||(tick>=600&&tick<620)||(tick>=680&&tick<700)?2:tick>=760&&tick<780?1:tick>=840&&tick<1340&&(tick-840)%80<20?2:tick>=1400&&tick<2220&&(tick-1400)%100<20?2:0;if(keys!=last_keys){key_until=cyc+4000000;last_keys=keys;g_t7_halted=0;}t7_set_buttons(keys);
while(cyc<tick*40000ull){g_t7_keyact=cyc<key_until?(unsigned)(key_until-cyc):0;unsigned c;
if(g_t7_halted&&(g_cpureg[0x73]|g_cpureg[0x74]))g_t7_halted=0;
if(g_t7_halted){unsigned long long next=tick*40000ull;if(video<next)next=video;if(tbh<next)next=tbh;if(cyc+(g_snd_next>0?g_snd_next:1)<next)next=cyc+(g_snd_next>0?g_snd_next:1);c=(unsigned)(next-cyc);if(!c)c=1;}else c=t7_cpu_step();cyc+=c;if((g_snd_next-=(int)c)<=0)t7_snd_service();
if(cyc>=video){video+=4000000/60;t7_fire_irq(10);t7_fire_nmi(6);}if(cyc>=tbh){tbh+=2000000;t7_fire_irq(13);}}
if(tick%100==0){unsigned char c[7]={g_cpu.pc&255,g_cpu.pc>>8,g_cpu.a,g_cpu.x,g_cpu.y,g_cpu.s,t7_dbg_p()};fwrite(c,1,7,f);fwrite(g_wram,1,1536,f);fwrite(g_dram,1,512,f);fwrite(g_cpureg,1,256,f);}}fclose(f);return 0;}
