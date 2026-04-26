#include "device_usb.h"
#include "device_everdrive.h"
#include <string.h>
#include <thread>
#include <chrono>

typedef struct 
{
    SerialDevice* device;
    USBHandle     handle;
    uint32_t      bytes_written;
    uint32_t      bytes_read;
} WiiHandle;


/*==============================
    device_test_wii
    Checks whether the device passed as an argument is a Wii
    @param  A pointer to the cart context
    @param  A pointer to a USB device
    @return DEVICEERR_OK if the cart is a Wii,
            DEVICEERR_NOTCART if it isn't,
            Any other device error if problems ocurred
==============================*/

DeviceError device_test_wii(CartDevice* cart, USB_DeviceInfoListNode* device_info)
{
    // Look for a single channel FTDI USB->serial adapter that isn't a flashcart
    if
    (
        strcmp(device_info->description, "FT245R USB FIFO") != 0 &&    // Everdrive, may have false positives
        strcmp(device_info->description, "64drive USB device") != 0 && // 64drive
        strcmp(device_info->description, "SC64") != 0 &&               // Summercart64
        (
            device_info->id == 0x04036001 || // FT232R (used by Everdrive)
            device_info->id == 0x04036014 || // FT232H (used by 64drive HW2 and Summercart)
            device_info->id == 0x04036015    // FT230X
        )
    )
    {
        SerialDevice* usbdevice = (SerialDevice*)malloc(sizeof(SerialDevice));
        usbdevice->vid = (uint16_t)(device_info->id >> 16);
        usbdevice->pid = (uint16_t)(device_info->id & 0xFFFF);
        memcpy(usbdevice->serial, device_info->serial, sizeof(device_info->serial));
        memcpy(usbdevice->description, device_info->description, sizeof(device_info->description));

        WiiHandle* fthandle = (WiiHandle*)malloc(sizeof(WiiHandle));
        fthandle->device = usbdevice;
        cart->structure = fthandle;
        return DEVICEERR_OK;
    }

    // Could not find the flashcart
    return DEVICEERR_NOTCART;
}


/*==============================
    device_open_wii
    Opens the USB pipe
    @param  A pointer to the cart context
    @return The device error, or OK
==============================*/

DeviceError device_open_wii(CartDevice* cart)
{
    WiiHandle* fthandle = (WiiHandle*) cart->structure;

    // Open the cart
    if (device_usb_open(fthandle->device, &fthandle->handle) != USB_OK || fthandle->handle == NULL)
        return DEVICEERR_CANTOPEN;

    // Reset the cart
    if (device_usb_resetdevice(fthandle->handle) != USB_OK)
        return DEVICEERR_RESETFAIL;
    if (device_usb_settimeouts(fthandle->handle, 500, 500) != USB_OK)
        return DEVICEERR_TIMEOUTSETFAIL;
    if (device_usb_purge(fthandle->handle, USB_PURGE_RX | USB_PURGE_TX) != USB_OK)
        return DEVICEERR_PURGEFAIL;
    if (device_usb_setbaudrate(fthandle->handle, 115200) != USB_OK)
        return DEVICEERR_TXREPLYMISMATCH;
    // Line properties 8N1
    if (device_usb_setdatacharacteristics(fthandle->handle, 8, 0, 0) != USB_OK)
        return DEVICEERR_BITMODEFAIL_SYNCFIFO;

    // Ok
    return DEVICEERR_OK;
}


/*==============================
    device_senddata_wii
    Sends data to the Wii
    @param  A pointer to the cart context
    @param  The datatype that is being sent
    @param  A buffer containing said data
    @param  The size of the data
    @return The device error, or OK
==============================*/

DeviceError device_senddata_wii(CartDevice* cart, USBDataType datatype, byte* data, uint32_t size)
{
    WiiHandle* fthandle = (WiiHandle*)cart->structure;
    uint32_t header;
    // Pad to alignment on 4-byte boundary + 2 bytes to
    // account for FTDI status bites on console side
    uint32_t newsize = ALIGN(size, 2) + 4;
    if (newsize % 4 == 0) newsize += 2;
    byte*    datacopy = NULL;
    uint32_t bytes_done = 0;
    uint32_t bytes_left = newsize;

    // Put in the DMA header along with length and type information in the buffer
    header = (size & 0xFFFFFF) | (((uint32_t)datatype) << 24);

    // Copy the data onto a temp variable
    datacopy = (byte*) calloc(newsize, 1);
    if (datacopy == NULL)
        return DEVICEERR_MALLOCFAIL;
    memcpy(datacopy+4, data, size);
    datacopy[0] = (header >> 24) & 0xFF;
    datacopy[1] = (header >> 16) & 0xFF;
    datacopy[2] = (header >> 8)  & 0xFF;
    datacopy[3] = header & 0xFF;

    // Send the data in chunks
    // Wii USB hardware buffer is 64 bytes, but needs space for FTDI status bytes
    device_setuploadprogress(0.0f);
    while (bytes_left > 0)
    {
        uint32_t bytes_do = 62;
        if (bytes_left < 62)
            bytes_do = bytes_left;
        if (device_usb_write(fthandle->handle, datacopy+bytes_done, bytes_do, &fthandle->bytes_written) != USB_OK)
            return DEVICEERR_WRITEFAIL;
        bytes_left -= fthandle->bytes_written;
        bytes_done += fthandle->bytes_written;
        device_setuploadprogress((((float)bytes_done)/((float)newsize))*100.0f);
    }

    // Free used up resources
    device_setuploadprogress(100.0f);
    free(datacopy);
    return DEVICEERR_OK;
}


/*==============================
    device_sendrawdata_wii
    Sends raw data to the Wii, no headers/footers
    @param  A pointer to the cart context
    @param  A buffer containing said data
    @param  The size of the data
    @return The device error, or OK
==============================*/

DeviceError device_sendrawdata_wii(CartDevice* cart, byte* data, uint32_t size)
{
    WiiHandle* fthandle = (WiiHandle*)cart->structure;
    uint32_t bytes_done = 0;
    uint32_t bytes_left = size;

    // Send the data in chunks
    // Wii USB hardware buffer is 64 bytes, but needs space for FTDI status bytes
    while (bytes_left > 0)
    {
        uint32_t bytes_do = 62;
        if (bytes_left < 62)
            bytes_do = bytes_left;
        if (device_usb_write(fthandle->handle, data+bytes_done, bytes_do, &fthandle->bytes_written) != USB_OK)
            return DEVICEERR_WRITEFAIL;
        bytes_left -= fthandle->bytes_written;
        bytes_done += fthandle->bytes_written;
    }

    return DEVICEERR_OK;
}


/*==============================
    device_receivedata_wii
    Receives data from the Wii
    @param  A pointer to the cart context
    @param  A pointer to an 32-bit value where
            the received data header will be
            stored.
    @param  A pointer to a byte buffer pointer
            where the data will be malloc'ed into.
    @return The device error, or OK
==============================*/

DeviceError device_receivedata_wii(CartDevice* cart, uint32_t* dataheader, byte** buff)
{
    WiiHandle* fthandle = (WiiHandle*)cart->structure;
    uint32_t size;
    uint32_t alignment = 4;

    // First, check if we have data to read
    if (device_usb_getqueuestatus(fthandle->handle, &size) != USB_OK)
        return DEVICEERR_POLLFAIL;

    // If we do, accounting for header min size
    if (size >= 4)
    {
        uint32_t dataread = 0;
        uint32_t totalread = 0;
        uint32_t offset = 4;
        byte     temp[4];

        // Get information about the incoming data and store it in dataheader
        if (device_usb_read(fthandle->handle, temp, 4, &fthandle->bytes_read) != USB_OK)
        {
            return DEVICEERR_BADHEADER;
        }
        (*dataheader) = swap_endian(temp[3] << 24 | temp[2] << 16 | temp[1] << 8 | temp[0]);
        totalread += fthandle->bytes_read;

        // Read the data into the buffer, in 512 byte chunks
        size = (*dataheader) & 0x00FFFFFF;
        (*buff) = (byte*)malloc(size);
        if ((*buff) == NULL)
            return DEVICEERR_MALLOCFAIL;

        // Do in 62 byte chunks because the Wii does it in 64 byte chunks with status bytes
        device_setuploadprogress(0.0f);
        while (dataread < size)
        {
            uint32_t readamount = size-dataread;
            if (readamount > 62 - offset)
                readamount = 62 - offset;
            if (device_usb_read(fthandle->handle, (*buff)+dataread, readamount, &fthandle->bytes_read) != USB_OK)
            {
                free((*buff));
                return DEVICEERR_READFAIL;
            }
            totalread += fthandle->bytes_read;
            dataread += fthandle->bytes_read;
            offset = 0;
            device_setuploadprogress((((float)dataread)/((float)size))*100.0f);
        }

        // Ensure 4 byte alignment by reading X amount of bytes needed
        if (totalread % alignment != 0)
        {
            byte* tempbuff = (byte*)malloc(alignment*sizeof(byte));
            int left = alignment - (totalread % alignment);
            if (device_usb_read(fthandle->handle, tempbuff, left, &fthandle->bytes_read) != USB_OK)
            {
                free(tempbuff);
                return DEVICEERR_BADPADDING;
            }
            free(tempbuff);
        }
        device_setuploadprogress(100.0f);
    }
    else
    {
        (*dataheader) = 0;
        (*buff) = NULL;
    }

    // All's good
    return DEVICEERR_OK;
}


/*==============================
    device_close_wii
    Closes the USB pipe
    @param  A pointer to the cart context
    @return The device error, or OK
==============================*/

DeviceError device_close_wii(CartDevice* cart)
{
    WiiHandle* fthandle = (WiiHandle*) cart->structure;
    if (device_usb_close(fthandle->handle) != USB_OK)
        return DEVICEERR_CLOSEFAIL;
    free(fthandle->device);
    free(fthandle);
    cart->structure = NULL;
    return DEVICEERR_OK;
}

/*==============================
    device_sendrom_everdrive
    ROM transfers unsupported
    on Wii, always returns an error.
    @param A pointer to the cart context
    @param A pointer to the ROM to send
    @param The size of the ROM
    @return The device error, or OK
==============================*/

DeviceError device_sendrom_wii(CartDevice* cart, byte* rom, uint32_t size)
{
    (void)cart; // Ignore unused paramater warning
    (void)rom;
    (void)size;
    return DEVICEERR_NOTCART;
}


/*==============================
    device_testdebug_everdrive
    Checks whether this cart can use debug mode
    @param A pointer to the cart context
    @returns The device error, or OK.
==============================*/

DeviceError device_testdebug_wii(CartDevice* cart)
{
    (void)cart; // Ignore unused paramater warning
    return DEVICEERR_NOTCART;
}


/*==============================
    device_maxromsize_wii
    ROM transfers unsupported
    on Wii, always returns 0 bytes.
    @return The max ROM size
==============================*/

uint32_t device_maxromsize_wii()
{
    return 0;
}


/*==============================
    device_rompadding_wii
    ROM transfers unsupported
    on Wii, always returns 0 bytes.
    @param  The current ROM size
    @return The correct ROM size 
            for uploading.
==============================*/

uint32_t device_rompadding_wii(uint32_t romsize)
{
    (void)(romsize); // Ignore unused paramater warning
    return 0;
}


/*==============================
    device_explicitcic_wii
    ROM transfers unsupported
    on Wii, always returns false.
    @param  The 4KB bootcode
    @return Whether the CIC was changed
==============================*/

bool device_explicitcic_wii(byte* bootcode)
{
    (void)(bootcode); // Ignore unused paramater warning
    return false;
}