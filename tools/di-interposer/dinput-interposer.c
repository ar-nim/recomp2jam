/* dinput-interposer.c — run-scoped DirectInput stand-in for the 7.55 exe (#699).
 *
 * The exe imports DirectInputCreateA only (OTwo.exe 7.55, sha256
 * 83a04541...72c823b) and reaches keyboard data through device vtable slots
 * +0x24 (GetDeviceState, 0x100 bytes) and +0x28 (GetDeviceData, 16-byte
 * records). This DLL fronts that entry point and answers both slots from
 * a scripted DIK feed; no original bytes are read or shipped here.
 *
 * Feed format is produced by the `di-interposer-feed` Rust driver:
 *   16-byte header: "O2DI1701", u32 version, u32 entry count
 *   16-byte entries: u64 Unix-ms target, u32 (DIK | pressed<<8), u32 reserved
 * All integers are little-endian. A pressed flag emits 0x80 in the state
 * byte and in the buffered record's second DWORD, matching the exe's tests.
 */
#define WIN32_LEAN_AND_MEAN
#include <dinput.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

#define KEY_STATE_BYTES 0x100u
#define BUFFERED_RECORD_BYTES 16u
#define MAX_BUFFERED_EVENTS 16u
#define PENDING_CAPACITY 512u
#define FEED_HEADER_BYTES 16u
#define FEED_RECORD_BYTES 16u
#define FEED_MAX_ENTRIES 100000u
#define FEED_VERSION 1u
#define PRESSED_BIT 0x80u
#define WORKER_INTERVAL_MS 5u

typedef enum FeedMode {
    FEED_MODE_BOTH = 0,
    FEED_MODE_STATE = 1,
    FEED_MODE_EVENTS = 2
} FeedMode;

typedef struct FeedEntry {
    uint64_t target_ms;
    uint8_t dik;
    uint8_t pressed;
} FeedEntry;

typedef struct BufferedEvent {
    uint32_t dik;
    uint32_t flags;
    uint32_t elapsed_ms;
    uint32_t sequence;
} BufferedEvent;

typedef struct ScriptedFeed {
    FeedEntry *entries;
    size_t count;
    size_t cursor;
    uint64_t origin_ms;
    BufferedEvent pending[PENDING_CAPACITY];
    size_t pending_head;
    size_t pending_count;
    uint32_t sequence;
    unsigned char key_state[KEY_STATE_BYTES];
    HANDLE notify_event;
    CRITICAL_SECTION lock;
    FeedMode mode;
    int loaded;
} ScriptedFeed;

typedef struct DeviceVtable DeviceVtable;
typedef struct ScriptedDevice {
    const DeviceVtable *vtable;
} ScriptedDevice;

struct DeviceVtable {
    HRESULT(WINAPI *query_interface)(ScriptedDevice *, REFIID, void **);
    ULONG(WINAPI *add_ref)(ScriptedDevice *);
    ULONG(WINAPI *release)(ScriptedDevice *);
    HRESULT(WINAPI *get_capabilities)(ScriptedDevice *, LPDIDEVCAPS);
    HRESULT(WINAPI *enum_objects)(ScriptedDevice *, LPDIENUMDEVICEOBJECTSCALLBACKA,
                                  LPVOID, DWORD);
    HRESULT(WINAPI *get_property)(ScriptedDevice *, REFGUID, LPDIPROPHEADER);
    HRESULT(WINAPI *set_property)(ScriptedDevice *, REFGUID, LPCDIPROPHEADER);
    HRESULT(WINAPI *acquire)(ScriptedDevice *);
    HRESULT(WINAPI *unacquire)(ScriptedDevice *);
    HRESULT(WINAPI *get_device_state)(ScriptedDevice *, DWORD, LPVOID);
    HRESULT(WINAPI *get_device_data)(ScriptedDevice *, DWORD, LPDIDEVICEOBJECTDATA,
                                     LPDWORD, DWORD);
    HRESULT(WINAPI *set_data_format)(ScriptedDevice *, LPCDIDATAFORMAT);
    HRESULT(WINAPI *set_event_notification)(ScriptedDevice *, HANDLE);
    HRESULT(WINAPI *set_cooperative_level)(ScriptedDevice *, HWND, DWORD);
    HRESULT(WINAPI *get_object_info)(ScriptedDevice *, LPDIDEVICEOBJECTINSTANCEA,
                                     DWORD, DWORD);
    HRESULT(WINAPI *get_device_info)(ScriptedDevice *, LPDIDEVICEINSTANCEA);
    HRESULT(WINAPI *run_control_panel)(ScriptedDevice *, HWND, DWORD);
    HRESULT(WINAPI *initialize)(ScriptedDevice *, HINSTANCE, DWORD, REFGUID);
    HRESULT(WINAPI *create_effect)(ScriptedDevice *, REFGUID, LPCDIEFFECT,
                                   LPDIRECTINPUTEFFECT *, LPUNKNOWN);
    HRESULT(WINAPI *enum_effects)(ScriptedDevice *, LPDIENUMEFFECTSCALLBACKA,
                                  LPVOID, DWORD);
    HRESULT(WINAPI *get_effect_info)(ScriptedDevice *, LPDIEFFECTINFOA, REFGUID);
    HRESULT(WINAPI *get_force_feedback_state)(ScriptedDevice *, LPDWORD);
    HRESULT(WINAPI *send_force_feedback_command)(ScriptedDevice *, DWORD);
    HRESULT(WINAPI *enum_created_effect_objects)(ScriptedDevice *,
        LPDIENUMCREATEDEFFECTOBJECTSCALLBACK, LPVOID, DWORD);
    HRESULT(WINAPI *escape)(ScriptedDevice *, LPDIEFFESCAPE);
    HRESULT(WINAPI *poll)(ScriptedDevice *);
    HRESULT(WINAPI *send_device_data)(ScriptedDevice *, DWORD,
                                      LPCDIDEVICEOBJECTDATA, LPDWORD, DWORD);
    HRESULT(WINAPI *enum_effects_in_file)(ScriptedDevice *, LPCSTR,
                                          LPDIENUMEFFECTSINFILECALLBACK, LPVOID, DWORD);
    HRESULT(WINAPI *write_effect_to_file)(ScriptedDevice *, LPCSTR, DWORD,
                                          LPDIFILEEFFECT, DWORD);
};

typedef struct DirectInputVtable {
    HRESULT(WINAPI *query_interface)(LPDIRECTINPUTA, REFIID, void **);
    ULONG(WINAPI *add_ref)(LPDIRECTINPUTA);
    ULONG(WINAPI *release)(LPDIRECTINPUTA);
    HRESULT(WINAPI *create_device)(LPDIRECTINPUTA, REFGUID,
                                   LPDIRECTINPUTDEVICEA *, LPUNKNOWN);
    HRESULT(WINAPI *enum_devices)(LPDIRECTINPUTA, DWORD, LPDIENUMDEVICESCALLBACKA,
                                  LPVOID, DWORD);
    HRESULT(WINAPI *get_device_status)(LPDIRECTINPUTA, REFGUID);
    HRESULT(WINAPI *run_control_panel)(LPDIRECTINPUTA, HWND, DWORD);
    HRESULT(WINAPI *initialize)(LPDIRECTINPUTA, HINSTANCE, DWORD);
    HRESULT(WINAPI *find_device)(LPDIRECTINPUTA, REFGUID, LPCSTR, LPGUID);
} DirectInputVtable;

typedef struct ScriptedDirectInput {
    const DirectInputVtable *vtable;
} ScriptedDirectInput;

typedef char device_state_slot_is_plus_0x24[
    (offsetof(DeviceVtable, get_device_state) == 0x24) ? 1 : -1];
typedef char device_data_slot_is_plus_0x28[
    (offsetof(DeviceVtable, get_device_data) == 0x28) ? 1 : -1];
typedef char device_create_slot_is_plus_0x0c[
    (offsetof(DirectInputVtable, create_device) == 0x0c) ? 1 : -1];
typedef char buffered_record_is_16_bytes[
    (sizeof(BufferedEvent) == BUFFERED_RECORD_BYTES) ? 1 : -1];

static ScriptedFeed scripted_feed;
static ScriptedDevice keyboard_device;
static ScriptedDevice mouse_device;
static ScriptedDirectInput direct_input_object;
static const DeviceVtable device_vtable;
static const DirectInputVtable direct_input_vtable;

static uint64_t read_le64(const unsigned char *bytes) {
    uint64_t value = 0;
    for (unsigned int index = 8; index > 0; index--) {
        value = (value << 8) | bytes[index - 1];
    }
    return value;
}

static uint32_t read_le32(const unsigned char *bytes) {
    uint32_t value = 0;
    for (unsigned int index = 4; index > 0; index--) {
        value = (value << 8) | bytes[index - 1];
    }
    return value;
}

static uint64_t divide_by_10000(uint64_t value) {
    uint64_t quotient = 0;
    uint64_t remainder = 0;
    for (unsigned int bit = 64; bit > 0; bit--) {
        remainder = (remainder << 1) | ((value >> (bit - 1)) & 1u);
        if (remainder >= 10000u) {
            remainder -= 10000u;
            quotient |= 1ULL << (bit - 1);
        }
    }
    return quotient;
}

static uint64_t unix_milliseconds(void) {
    FILETIME file_time;
    GetSystemTimeAsFileTime(&file_time);
    uint64_t ticks = ((uint64_t)file_time.dwHighDateTime << 32) |
                     file_time.dwLowDateTime;
    return divide_by_10000(ticks) - 11644473600000ULL;
}

static FeedMode configured_mode(void) {
    const char *name = getenv("DI_INTERPOSER_MODE");
    if (name != NULL && strcmp(name, "state") == 0) {
        return FEED_MODE_STATE;
    }
    if (name != NULL && strcmp(name, "events") == 0) {
        return FEED_MODE_EVENTS;
    }
    return FEED_MODE_BOTH;
}

static int read_feed_entries(const char *path, FeedEntry **out_entries,
                             size_t *out_count) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return 0;
    }
    unsigned char header[FEED_HEADER_BYTES];
    int ok = fread(header, 1, sizeof header, file) == sizeof header &&
             memcmp(header, "O2DI1701", 8) == 0 &&
             read_le32(header + 8) == FEED_VERSION;
    uint32_t count = ok ? read_le32(header + 12) : 0;
    ok = ok && count > 0 && count <= FEED_MAX_ENTRIES;
    FeedEntry *entries = ok ? malloc((size_t)count * sizeof *entries) : NULL;
    if (entries == NULL) {
        fclose(file);
        return 0;
    }
    unsigned char record[FEED_RECORD_BYTES];
    size_t loaded = 0;
    while (ok && loaded < count &&
           fread(record, 1, sizeof record, file) == sizeof record) {
        uint32_t code = read_le32(record + 8);
        uint8_t dik = (uint8_t)(code & 0xffu);
        if (dik == 0 || (code & 0x100u) == 0 ||
            (dik & PRESSED_BIT) != 0) {
            ok = 0;
            break;
        }
        entries[loaded].target_ms = read_le64(record);
        entries[loaded].dik = dik;
        entries[loaded].pressed = (uint8_t)((code >> 8) & 1u);
        loaded++;
    }
    ok = ok && loaded == count;
    fclose(file);
    if (!ok) {
        free(entries);
        return 0;
    }
    *out_entries = entries;
    *out_count = loaded;
    return 1;
}

static void signal_notification(ScriptedFeed *feed) {
    if (feed->notify_event != NULL && feed->pending_count > 0) {
        SetEvent(feed->notify_event);
    }
}

static void enqueue_event(ScriptedFeed *feed, const FeedEntry *entry) {
    if (feed->pending_count == PENDING_CAPACITY) {
        return;
    }
    size_t slot = (feed->pending_head + feed->pending_count) % PENDING_CAPACITY;
    BufferedEvent *event = &feed->pending[slot];
    event->dik = entry->dik;
    event->flags = entry->pressed ? PRESSED_BIT : 0;
    event->elapsed_ms = (uint32_t)(entry->target_ms - feed->origin_ms);
    event->sequence = feed->sequence++;
    feed->pending_count++;
    signal_notification(feed);
}

static void apply_due_entries(ScriptedFeed *feed, uint64_t now) {
    if (!feed->loaded) {
        return;
    }
    EnterCriticalSection(&feed->lock);
    while (feed->cursor < feed->count &&
           feed->entries[feed->cursor].target_ms <= now) {
        FeedEntry *entry = &feed->entries[feed->cursor];
        feed->key_state[entry->dik] = entry->pressed ? PRESSED_BIT : 0;
        if (feed->mode != FEED_MODE_STATE) {
            enqueue_event(feed, entry);
        }
        feed->cursor++;
    }
    LeaveCriticalSection(&feed->lock);
}

static void load_scripted_feed(void) {
    scripted_feed.mode = configured_mode();
    const char *path = getenv("DI_INTERPOSER_SCHEDULE");
    if (path == NULL || *path == '\0') {
        return;
    }
    FeedEntry *entries = NULL;
    size_t count = 0;
    if (!read_feed_entries(path, &entries, &count)) {
        return;
    }
    scripted_feed.entries = entries;
    scripted_feed.count = count;
    scripted_feed.origin_ms = entries[0].target_ms;
    scripted_feed.loaded = 1;
}

static DWORD WINAPI feed_worker(LPVOID unused) {
    (void)unused;
    while (scripted_feed.loaded) {
        apply_due_entries(&scripted_feed, unix_milliseconds());
        Sleep(WORKER_INTERVAL_MS);
    }
    return 0;
}

static void start_feed_worker(void) {
    if (!scripted_feed.loaded || scripted_feed.mode == FEED_MODE_STATE) {
        return;
    }
    HANDLE thread = CreateThread(NULL, 0, feed_worker, NULL, 0, NULL);
    if (thread != NULL) {
        CloseHandle(thread);
    }
}

/* The feed epoch is known only after the song clock starts, so the file may
 * not exist when DirectInputCreateA runs. Retry until the boot leg writes it. */
static void load_feed_when_available(void) {
    if (!scripted_feed.loaded) {
        load_scripted_feed();
        start_feed_worker();
    }
}

static HRESULT WINAPI device_query_interface(ScriptedDevice *device,
                                             REFIID unused, void **out) {
    (void)device;
    (void)unused;
    if (out == NULL) {
        return DIERR_INVALIDPARAM;
    }
    *out = device;
    return DI_OK;
}

static ULONG WINAPI device_add_ref(ScriptedDevice *device) {
    (void)device;
    return 1;
}

static ULONG WINAPI device_release(ScriptedDevice *device) {
    (void)device;
    return 0;
}

/* Each unused vtable slot keeps the exact DirectInput signature: stdcall
 * callees must pop the same argument bytes the exe pushes. */
static HRESULT WINAPI accept_device(ScriptedDevice *) { return DI_OK; }
static HRESULT WINAPI accept_flags(ScriptedDevice *, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_device_caps(ScriptedDevice *, LPDIDEVCAPS) {
    return DI_OK;
}
static HRESULT WINAPI accept_object_enum(ScriptedDevice *,
    LPDIENUMDEVICEOBJECTSCALLBACKA, LPVOID, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_property(ScriptedDevice *, REFGUID, LPDIPROPHEADER) {
    return DI_OK;
}
static HRESULT WINAPI accept_data_format(ScriptedDevice *, LPCDIDATAFORMAT) {
    return DI_OK;
}
static HRESULT WINAPI accept_window_flags(ScriptedDevice *, HWND, DWORD) {
    return DI_OK;
}
static HRESULT WINAPI accept_object_info(ScriptedDevice *,
    LPDIDEVICEOBJECTINSTANCEA, DWORD, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_device_info(ScriptedDevice *, LPDIDEVICEINSTANCEA) {
    return DI_OK;
}
static HRESULT WINAPI accept_initialize(ScriptedDevice *, HINSTANCE, DWORD,
                                        REFGUID) { return DI_OK; }
static HRESULT WINAPI accept_create_effect(ScriptedDevice *, REFGUID, LPCDIEFFECT,
    LPDIRECTINPUTEFFECT *, LPUNKNOWN) { return DI_OK; }
static HRESULT WINAPI accept_enum_effects(ScriptedDevice *,
    LPDIENUMEFFECTSCALLBACKA, LPVOID, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_effect_info(ScriptedDevice *, LPDIEFFECTINFOA,
                                         REFGUID) { return DI_OK; }
static HRESULT WINAPI accept_force_state(ScriptedDevice *, LPDWORD) {
    return DI_OK;
}
static HRESULT WINAPI accept_created_effects(ScriptedDevice *,
    LPDIENUMCREATEDEFFECTOBJECTSCALLBACK, LPVOID, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_escape(ScriptedDevice *, LPDIEFFESCAPE) {
    return DI_OK;
}
static HRESULT WINAPI accept_send_device_data(ScriptedDevice *, DWORD,
    LPCDIDEVICEOBJECTDATA, LPDWORD, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_effects_in_file(ScriptedDevice *, LPCSTR,
    LPDIENUMEFFECTSINFILECALLBACK, LPVOID, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_write_effect(ScriptedDevice *, LPCSTR, DWORD,
    LPDIFILEEFFECT, DWORD) { return DI_OK; }

static HRESULT WINAPI get_device_state(ScriptedDevice *device, DWORD size,
                                       LPVOID destination) {
    if (destination == NULL || size == 0) {
        return DIERR_INVALIDPARAM;
    }
    DWORD copied = size < KEY_STATE_BYTES ? size : KEY_STATE_BYTES;
    unsigned char *bytes = destination;
    if (device != &keyboard_device) {
        for (DWORD index = 0; index < copied; index++) {
            bytes[index] = 0;
        }
        return DI_OK;
    }
    load_feed_when_available();
    apply_due_entries(&scripted_feed, unix_milliseconds());
    EnterCriticalSection(&scripted_feed.lock);
    for (DWORD index = 0; index < copied; index++) {
        bytes[index] = scripted_feed.key_state[index];
    }
    LeaveCriticalSection(&scripted_feed.lock);
    return DI_OK;
}

static DWORD drain_events(LPDIDEVICEOBJECTDATA destination, DWORD capacity) {
    if (destination == NULL || capacity == 0) {
        return 0;
    }
    DWORD written = 0;
    EnterCriticalSection(&scripted_feed.lock);
    while (written < capacity && scripted_feed.pending_count > 0) {
        BufferedEvent *event =
            &scripted_feed.pending[scripted_feed.pending_head];
        scripted_feed.pending_head =
            (scripted_feed.pending_head + 1) % PENDING_CAPACITY;
        scripted_feed.pending_count--;
        destination[written].dwOfs = event->dik;
        destination[written].dwData = event->flags;
        destination[written].dwTimeStamp = event->elapsed_ms;
        destination[written].dwSequence = event->sequence;
        written++;
    }
    signal_notification(&scripted_feed);
    LeaveCriticalSection(&scripted_feed.lock);
    return written;
}

static HRESULT WINAPI get_device_data(ScriptedDevice *device, DWORD record_size,
                                      LPDIDEVICEOBJECTDATA destination,
                                      LPDWORD in_out, DWORD flags) {
    (void)device;
    (void)flags;
    if (in_out == NULL || record_size < BUFFERED_RECORD_BYTES ||
        (destination == NULL && *in_out > 0)) {
        return DIERR_INVALIDPARAM;
    }
    load_feed_when_available();
    apply_due_entries(&scripted_feed, unix_milliseconds());
    DWORD capacity = *in_out;
    if (capacity > MAX_BUFFERED_EVENTS) {
        capacity = MAX_BUFFERED_EVENTS;
    }
    *in_out = drain_events(destination, capacity);
    return DI_OK;
}

static HRESULT WINAPI set_event_notification(ScriptedDevice *device,
                                             HANDLE event) {
    (void)device;
    EnterCriticalSection(&scripted_feed.lock);
    scripted_feed.notify_event = event;
    LeaveCriticalSection(&scripted_feed.lock);
    return DI_OK;
}

static HRESULT WINAPI set_property(ScriptedDevice *device, REFGUID property,
                                   LPCDIPROPHEADER value) {
    (void)device;
    (void)property;
    (void)value;
    return DI_OK;
}

static const DeviceVtable device_vtable = {
    device_query_interface,
    device_add_ref,
    device_release,
    accept_device_caps,
    accept_object_enum,
    accept_property,
    set_property,
    accept_device,
    accept_device,
    get_device_state,
    get_device_data,
    accept_data_format,
    set_event_notification,
    accept_window_flags,
    accept_object_info,
    accept_device_info,
    accept_window_flags,
    accept_initialize,
    accept_create_effect,
    accept_enum_effects,
    accept_effect_info,
    accept_force_state,
    accept_flags,
    accept_created_effects,
    accept_escape,
    accept_device,
    accept_send_device_data,
    accept_effects_in_file,
    accept_write_effect
};

static HRESULT WINAPI direct_input_query_interface(LPDIRECTINPUTA object,
                                                   REFIID unused, void **out) {
    (void)object;
    (void)unused;
    if (out == NULL) {
        return DIERR_INVALIDPARAM;
    }
    *out = &direct_input_object;
    return DI_OK;
}

static ULONG WINAPI direct_input_add_ref(LPDIRECTINPUTA object) {
    (void)object;
    return 1;
}

static ULONG WINAPI direct_input_release(LPDIRECTINPUTA object) {
    (void)object;
    return 0;
}

static HRESULT WINAPI accept_enum_devices(LPDIRECTINPUTA, DWORD,
    LPDIENUMDEVICESCALLBACKA, LPVOID, DWORD) { return DI_OK; }
static HRESULT WINAPI accept_device_status(LPDIRECTINPUTA, REFGUID) {
    return DI_OK;
}
static HRESULT WINAPI accept_direct_input_window(LPDIRECTINPUTA, HWND, DWORD) {
    return DI_OK;
}
static HRESULT WINAPI accept_direct_input_init(LPDIRECTINPUTA, HINSTANCE, DWORD) {
    return DI_OK;
}
static HRESULT WINAPI accept_find_device(LPDIRECTINPUTA, REFGUID, LPCSTR, LPGUID) {
    return DI_OK;
}

static ScriptedDevice *device_for_guid(REFGUID guid) {
    if (guid != NULL && guid->Data1 == 0x6f1d2b61) {
        return &keyboard_device;
    }
    return &mouse_device;
}

static HRESULT WINAPI create_device(LPDIRECTINPUTA object, REFGUID guid,
                                    LPDIRECTINPUTDEVICEA *out, LPUNKNOWN outer) {
    (void)object;
    (void)outer;
    if (out == NULL) {
        return DIERR_INVALIDPARAM;
    }
    *out = (LPDIRECTINPUTDEVICEA)device_for_guid(guid);
    return DI_OK;
}

static const DirectInputVtable direct_input_vtable = {
    direct_input_query_interface,
    direct_input_add_ref,
    direct_input_release,
    create_device,
    accept_enum_devices,
    accept_device_status,
    accept_direct_input_window,
    accept_direct_input_init,
    accept_find_device
};

static void initialize_objects(void) {
    keyboard_device.vtable = &device_vtable;
    mouse_device.vtable = &device_vtable;
    direct_input_object.vtable = &direct_input_vtable;
}

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID unused) {
    (void)instance;
    (void)unused;
    if (reason == DLL_PROCESS_ATTACH) {
        InitializeCriticalSection(&scripted_feed.lock);
        initialize_objects();
    } else if (reason == DLL_PROCESS_DETACH) {
        DeleteCriticalSection(&scripted_feed.lock);
        free(scripted_feed.entries);
        scripted_feed.entries = NULL;
    }
    return TRUE;
}

HRESULT WINAPI DirectInputCreateA(
    HINSTANCE instance, DWORD version, LPDIRECTINPUTA *out, LPUNKNOWN outer) {
    (void)instance;
    (void)version;
    (void)outer;
    if (out == NULL) {
        return DIERR_INVALIDPARAM;
    }
    *out = (LPDIRECTINPUTA)&direct_input_object;
    if (!scripted_feed.loaded) {
        load_scripted_feed();
        start_feed_worker();
    }
    return DI_OK;
}
