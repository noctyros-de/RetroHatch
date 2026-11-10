/* SPDX-License-Identifier: GPL-3.0-only
 * SPDX-FileCopyrightText: 2026 Noctyros
 * Test-only adapter for unchanged maragotchi/tamaemu CPU/memory/LCD sources.
 * Peripherals are deliberately unavailable: this is not an MCU/boot oracle. */
#include "emu.h"
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

static void unavailable(void) { fputs("Unexpected peripheral use in CPU/memory oracle\n",stderr);exit(90); }
uint8_t periph_read8(Emu *e,uint32_t a) {(void)e;(void)a;unavailable();return 0;}
uint32_t periph_read32(Emu *e,uint32_t a) {(void)e;(void)a;unavailable();return 0;}
void periph_write8(Emu *e,uint32_t a,uint8_t v) {(void)e;(void)a;(void)v;unavailable();}
void periph_write32(Emu *e,uint32_t a,uint32_t v) {(void)e;(void)a;(void)v;unavailable();}
void periph_ir_session_begin(Emu *e) {(void)e;unavailable();}
/* CPU-reset tests cover CPU fields only, not PN512 reset behavior. */
void pn512_power_on(Emu *e) {(void)e;}
void pn512_probe_set(Emu *e,int on) {(void)e;(void)on;unavailable();}
const char *disasm_sym_for(uint32_t pc) {(void)pc;return "";}
static uint64_t get(void) {unsigned long long x;if(scanf("%llx",&x)!=1)exit(91);return x;}
static uint32_t hash_bytes(uint32_t h,const uint8_t *p,size_t n) {while(n--)h=(h^*p++)*16777619u;return h;}
static uint32_t read_hash(uint32_t h,uint32_t x) {for(int i=0;i<4;i++){h=(h^(uint8_t)x)*16777619u;x>>=8;}return h;}
static uint32_t psr(Emu *e){return(e->psr_rest&~0xF1Fu)|e->f_n|(e->f_z<<1)|(e->f_v<<2)|(e->f_c<<3)|(e->f_ie<<4)|((uint32_t)(e->il&15)<<8);}
static void emit(uint64_t x) {printf(" %" PRIx64,x);}
int main(void) {
    char name[100];
    while(scanf("%99s",name)==1) {
        unsigned profile=(unsigned)get(),steps=(unsigned)get();uint32_t flags=(uint32_t)get();
        Emu *e=(Emu*)calloc(1,sizeof *e);if(!e)return 92;
        const DeviceProfile *dev=device_at(profile);if(!dev)return 93;e->dev=*dev;
        e->rom=(uint8_t*)malloc(dev->rom_size);if(!e->rom)return 94;memset(e->rom,255,dev->rom_size);
        uint32_t reset=0x1000;memcpy(e->rom+dev->ttbr_reset-dev->rom_base,&reset,4);
        e->pending_irq=(uint32_t)get();e->pending_level=(uint8_t)get();e->halted=get()!=0;e->halt_hard=get()!=0;e->wake_req=get()!=0;e->cmu.opt=(uint32_t)get();e->cycles=get();e->wake_at=get();
        for(int i=0;i<16;i++)e->r[i]=(uint32_t)get();e->sp=(uint32_t)get();e->alr=(uint32_t)get();e->ahr=(uint32_t)get();e->ttbr=(uint32_t)get();e->idir=(uint32_t)get();e->dbbr=(uint32_t)get();e->pc=(uint32_t)get();
        e->psr_rest=flags;e->f_n=flags&1;e->f_z=(flags>>1)&1;e->f_v=(flags>>2)&1;e->f_c=(flags>>3)&1;e->f_ie=(flags>>4)&1;e->il=(flags>>8)&15;
        for(unsigned i=0;i<dev->a0ram_size;i++)e->a0ram[i]=(uint8_t)(i*37+11);
        for(unsigned i=0;i<dev->ivram_size;i++)e->ivram[i]=(uint8_t)(i*17+3);
        for(unsigned i=0;i<dev->dstram_size;i++)e->dstram[i]=(uint8_t)(i*13+7);
        for(unsigned i=0;i<80;i++){uint32_t handler=0x1800;memcpy(e->a0ram+0x4000+i*4,&handler,4);}
        e->a0ram[0x1800]=0x75;e->a0ram[0x1801]=0x6c;
        unsigned count=(unsigned)get();uint32_t *code=(uint32_t*)malloc((count+1)*sizeof *code);
        for(unsigned i=0;i<count;i++){code[i]=(uint32_t)get();if(steps!=0xffff){e->a0ram[0x1000+i*2]=(uint8_t)code[i];e->a0ram[0x1001+i*2]=(uint8_t)(code[i]>>8);}}
        uint32_t reads=2166136261u;
        if(steps==0xffff) {
            for(unsigned i=0;i<count;i+=3){uint32_t kind=code[i],a=code[i+1],v=code[i+2];
                if(kind==0)reads=read_hash(reads,mem_read8(e,a));else if(kind==1)reads=read_hash(reads,mem_read16(e,a));else if(kind==2)reads=read_hash(reads,mem_read32(e,a));
                else if(kind==3)mem_write8(e,a,(uint8_t)v);else if(kind==4)mem_write16(e,a,(uint16_t)v);else if(kind==5)mem_write32(e,a,v);else return 95;
            }
        }else if(!steps)cpu_reset(e);else for(unsigned i=0;i<steps;i++)cpu_step(e);
        printf("%s",name);emit(e->stopped);
        for(int i=0;i<16;i++)emit(e->r[i]);
        emit(e->sp);emit(e->alr);emit(e->ahr);emit(e->ttbr);emit(e->idir);emit(e->dbbr);emit(e->pc);emit(psr(e));emit(e->psr_rest);emit(e->ext_count);emit(e->ext_val[0]);emit(e->ext_val[1]);
        emit(e->halted);emit(e->halt_hard);emit(e->cycles);emit(e->wake_at);emit(e->pending_irq);emit(e->pending_level);emit(e->halt_wakes);emit(e->wake_req);
        uint32_t ram=hash_bytes(2166136261u,e->a0ram,dev->a0ram_size);ram=hash_bytes(ram,e->ivram,dev->ivram_size);ram=hash_bytes(ram,e->dstram,dev->dstram_size);emit(ram);
        emit(e->flash_state);emit(e->flash_autoselect);emit(e->flash_busy);emit(e->flash_toggle);emit(e->flash_op_data);emit(e->flash_dirty);emit(e->flash_programs);emit(e->flash_erases);emit(e->last_io_read);emit(reads);
        uint32_t flash=hash_bytes(2166136261u,e->rom,1024);flash=hash_bytes(flash,e->rom+dev->rom_size-1024,1024);flash=hash_bytes(flash,e->rom+0x1234,16);flash=hash_bytes(flash,e->rom+0x20000,16);emit(flash);
        uint32_t *pixels=(uint32_t*)malloc(PANEL_W*PANEL_H*4);int w,h;lcd_render(e,pixels,&w,&h);emit(hash_bytes(2166136261u,(uint8_t*)pixels,(size_t)(w*h*4)));
        emit(e->lcd.cmd);emit(e->lcd.argc);emit(e->lcd.xs);emit(e->lcd.xe);emit(e->lcd.ys);emit(e->lcd.ye);emit(e->lcd.cx);emit(e->lcd.cy);emit(e->lcd.madctl);emit(e->lcd.colmod);emit(e->lcd.sleep_out);emit(e->lcd.disp_on);emit(e->lcd.inverted);emit(e->lcd.pix_n);emit(e->lcd.ramwr_bytes);emit(e->lcd.cmd_count);emit(e->lcd.frame_marker);
        emit(hash_bytes(2166136261u,(uint8_t*)e->lcd.gram,sizeof e->lcd.gram));
        emit(hash_bytes(2166136261u,e->lcd.args,16));
        emit(hash_bytes(2166136261u,e->lcd.pix_buf,3));
        emit(hash_bytes(2166136261u,e->lcd.gamma_pos,16));emit(hash_bytes(2166136261u,e->lcd.gamma_neg,16));
        emit(e->lcd.gp_n);emit(e->lcd.gn_n);emit(e->lcd.pwr_c0[0]);emit(e->lcd.pwr_c0[1]);emit(e->lcd.pwr_c2[0]);emit(e->lcd.vcom_c5[0]);emit(e->lcd.vcom_c6[0]);
        emit(e->lcd.c0_n);emit(e->lcd.c2_n);emit(e->lcd.c5_n);emit(e->lcd.c6_n);
        putchar('\n');free(pixels);free(code);free(e->rom);free(e);
    }return 0;
}
