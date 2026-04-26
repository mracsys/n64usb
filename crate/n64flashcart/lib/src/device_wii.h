#ifndef __DEVICE_WII_HEADER
#define __DEVICE_WII_HEADER

    #include "device.hpp"
    #include "device_usb.h"


    /*********************************
            Function Prototypes
    *********************************/

    DeviceError device_test_wii(CartDevice* cart, USB_DeviceInfoListNode* device_info);
    DeviceError device_open_wii(CartDevice* cart);
    DeviceError device_sendrom_wii(CartDevice* cart, byte* rom, uint32_t size);
    uint32_t    device_maxromsize_wii();
    uint32_t    device_rompadding_wii(uint32_t romsize);
    bool        device_explicitcic_wii(byte* bootcode);
    DeviceError device_testdebug_wii(CartDevice* cart);
    DeviceError device_senddata_wii(CartDevice* cart, USBDataType datatype, byte* data, uint32_t size);
    DeviceError device_sendrawdata_wii(CartDevice* cart, byte* data, uint32_t size);
    DeviceError device_receivedata_wii(CartDevice* cart, uint32_t* dataheader, byte** buff);
    DeviceError device_close_wii(CartDevice* cart);

#endif