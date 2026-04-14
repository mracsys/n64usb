#ifndef __DEVICE_HEADER
#define __DEVICE_HEADER

    typedef struct IUnknown IUnknown;

    #include <stdint.h>
    #include <stdlib.h>
    #include <stdio.h>
    #include <stdbool.h>


    /*********************************
                  Macros
    *********************************/

    #define USBPROTOCOL_LATEST PROTOCOL_VERSION2


    /*********************************
               Enumerations
    *********************************/

    typedef enum {
        CART_NONE      = 0,
        CART_64DRIVE1  = 1,
        CART_64DRIVE2  = 2,
        CART_EVERDRIVE = 3,
        CART_SC64      = 4,
        CART_GOPHER64  = 5,
        CART_WII       = 6,
    } CartType;

    typedef enum {
        CIC_NONE = -1,
        CIC_6101 = 0,
        CIC_6102 = 1,
        CIC_7101 = 2,
        CIC_7102 = 3,
        CIC_X103 = 4,
        CIC_X105 = 5,
        CIC_X106 = 6,
        CIC_5101 = 7,
        CIC_8303 = 8
    } CICType;

    typedef enum {
        SAVE_NONE         = 0,
        SAVE_EEPROM4K     = 1,
        SAVE_EEPROM16K    = 2,
        SAVE_SRAM256      = 3,
        SAVE_FLASHRAM     = 4,
        SAVE_SRAM768      = 5,
        SAVE_FLASHRAMPKMN = 6,
    } SaveType;

    typedef enum {
        DATATYPE_EMPTY           = 0x00,
        DATATYPE_TEXT            = 0x01,
        DATATYPE_RAWBINARY       = 0x02,
        DATATYPE_HEADER          = 0x03,
        DATATYPE_SCREENSHOT      = 0x04,
        DATATYPE_HEARTBEAT       = 0x05,
        DATATYPE_RDBPACKET       = 0x06,
        DATATYPE_TCPTEST         = 0x07,
        DATATYPE_ROMUPLOAD       = 0x08,
        DATATYPE_HANDSHAKE       = 0x09,
        DATATYPE_INGAME_STATE    = 0x0A,
        DATATYPE_SAVE_FILENAME   = 0x0B,
        DATATYPE_RESET           = 0x0C,
        DATATYPE_SEND_ITEM       = 0x0D,
        DATATYPE_ACK_MESSAGE     = 0x0E,
        DATATYPE_DUNGEON_REWARDS = 0x0F,
        DATATYPE_PLAYER_NAMES    = 0x10,
        DATATYPE_READ_MEMORY     = 0x11,
        DATATYPE_WRITE_MEMORY    = 0x12,
        DATATYPE_UNRECOVERABLE   = 0x13,
    } USBDataType;

    typedef enum {
        PROTOCOL_VERSION1   = 0x00, 
        PROTOCOL_VERSION2   = 0x02,
    } ProtocolVer;

    typedef enum {
/*  0 */DEVICEERR_OK = 0,
/*  1 */DEVICEERR_NOTCART,
/*  2 */DEVICEERR_USBBUSY,
/*  3 */DEVICEERR_NODEVICES,
/*  4 */DEVICEERR_CARTFINDFAIL,
/*  5 */DEVICEERR_CANTOPEN,
/*  6 */DEVICEERR_FILEREADFAIL,
/*  7 */DEVICEERR_RESETFAIL,
/*  8 */DEVICEERR_RESETPORTFAIL,
/*  9 */DEVICEERR_TIMEOUTSETFAIL,
/* 10 */DEVICEERR_PURGEFAIL,
/* 11 */DEVICEERR_READFAIL,
/* 12 */DEVICEERR_WRITEFAIL,
/* 13 */DEVICEERR_WRITEZERO,
/* 14 */DEVICEERR_CLOSEFAIL,
/* 15 */DEVICEERR_BITMODEFAIL_RESET,
/* 16 */DEVICEERR_BITMODEFAIL_SYNCFIFO,
/* 17 */DEVICEERR_SETDTRFAIL,
/* 18 */DEVICEERR_CLEARDTRFAIL,
/* 19 */DEVICEERR_GETMODEMSTATUSFAIL,
/* 20 */DEVICEERR_TXREPLYMISMATCH,
/* 21 */DEVICEERR_READCOMPSIGFAIL,
/* 22 */DEVICEERR_NOCOMPSIG,
/* 23 */DEVICEERR_READPACKSIZEFAIL,
/* 24 */DEVICEERR_BADPACKSIZE,
/* 25 */DEVICEERR_MALLOCFAIL,
/* 26 */DEVICEERR_UPLOADCANCELLED,
/* 27 */DEVICEERR_TIMEOUT,
/* 28 */DEVICEERR_POLLFAIL,
/* 29 */DEVICEERR_64D_BADCMP,
/* 30 */DEVICEERR_64D_8303USB,
/* 31 */DEVICEERR_64D_CANTDEBUG,
/* 32 */DEVICEERR_64D_BADDMA,
/* 33 */DEVICEERR_64D_DATATOOBIG,
/* 34 */DEVICEERR_SC64_CMDFAIL,
/* 35 */DEVICEERR_SC64_COMMFAIL,
/* 36 */DEVICEERR_SC64_CTRLRELEASEFAIL,
/* 37 */DEVICEERR_SC64_CTRLRESETFAIL,
/* 38 */DEVICEERR_SC64_FIRMWARECHECKFAIL,
/* 39 */DEVICEERR_SC64_FIRMWAREUNSUPPORTED,
    } DeviceError;


    /*********************************
                 Typedefs
    *********************************/

    typedef uint8_t byte;

    typedef struct {
        CartType    carttype;
        CICType     cictype;
        SaveType    savetype;
        ProtocolVer protocol;
        void*       structure;
    } CartDevice;

    typedef struct {
        uint16_t vid;
        uint16_t pid;
        char     serial[16];
        char     description[64];
    } SerialDevice;


    /*********************************
            Function Prototypes
    *********************************/

    // Main device functions
    void        device_initialize();
    DeviceError device_find();
    void        device_list(SerialDevice *devices, uint32_t *device_count);
    DeviceError device_connect(uint32_t id, char *serial);
    DeviceError device_open();
    uint32_t    device_getmaxromsize();
    uint32_t    device_rompadding(uint32_t romsize);
    bool        device_explicitcic();
    bool        device_isopen();
    DeviceError device_testdebug();
    DeviceError device_sendrom(FILE* rom, uint32_t filesize);
    DeviceError device_senddata(USBDataType datatype, byte* data, uint32_t size);
    DeviceError device_receivedata(uint32_t* dataheader, byte** buff);
    DeviceError device_close();

    // Device configuration
	bool     device_setrom(const char* path);
    void     device_setcart(CartType cart);
    void     device_setcic(CICType cic);
    void     device_setsave(SaveType save);
    char*    device_getrom();
    CartType device_getcart();
    CICType  device_getcic();
    SaveType device_getsave();

    // Upload related
    void  device_cancelupload();
    bool  device_uploadcancelled();
    void  device_setuploadprogress(float progress);
    float device_getuploadprogress();

    // Protocol version handling
    void        device_setprotocol(ProtocolVer version);
    ProtocolVer device_getprotocol();
    
    // Helper functions
    #define  SWAP(a, b) (((a) ^= (b)), ((b) ^= (a)), ((a) ^= (b))) // From https://graphics.stanford.edu/~seander/bithacks.html#SwappingValuesXOR
    #define  ALIGN(s, align) (((uint32_t)(s) + ((align)-1)) & ~((align)-1))
    uint32_t swap_endian(uint32_t val);
    uint32_t calc_padsize(uint32_t size);
    uint32_t romhash(byte* buff, uint32_t len);
    CICType  cic_from_bootcode(byte *bootcode);


#endif