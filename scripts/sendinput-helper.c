/* sendinput_helper.c — Wine-side SendInput injector for the exe trace player.
 *
 * The DirectDraw canvas in OTwo.exe never sees XTEST motion; this helper
 * runs inside Wine and delivers input through the Win32 SendInput queue
 * that the canvas actually polls (see docs/wine-boot.md, Attempts 7-9).
 *
 * Usage (one command per invocation):
 *   sendinput_helper.exe.so move X Y
 *   sendinput_helper.exe.so click X Y        # move, down, up
 *   sendinput_helper.exe.so down X Y         # move then left-down
 *   sendinput_helper.exe.so up X Y           # move then left-up
 *   sendinput_helper.exe.so key <name> down|up
 *   sendinput_helper.exe.so type <text>
 *
 * Key names: a-z, 0-9, space, return, escape, up, down, left, right.
 * All coordinates are in the native 800x600 canvas space; SetCursorPos
 * places the pointer there before the SendInput call.
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

static void join_game_desktop(void) {
    HDESK desk = OpenDesktopA("o2jam", 0, FALSE, MAXIMUM_ALLOWED);
    if (desk) {
        SetThreadDesktop(desk);
    }
    Sleep(100);
}

static void move_to(int x, int y) {
    SetCursorPos(x, y);
    Sleep(20);
}

static void left_down(void) {
    INPUT inp;
    ZeroMemory(&inp, sizeof(inp));
    inp.type = INPUT_MOUSE;
    inp.mi.dwFlags = MOUSEEVENTF_LEFTDOWN;
    SendInput(1, &inp, sizeof(inp));
}

static void left_up(void) {
    INPUT inp;
    ZeroMemory(&inp, sizeof(inp));
    inp.type = INPUT_MOUSE;
    inp.mi.dwFlags = MOUSEEVENTF_LEFTUP;
    SendInput(1, &inp, sizeof(inp));
}

static WORD vk_for_name(const char *name) {
    if (strlen(name) == 1) {
        char c = name[0];
        if (c >= 'a' && c <= 'z') return (WORD)(c - 'a' + 'A');
        if (c >= 'A' && c <= 'Z') return (WORD)c;
        if (c >= '0' && c <= '9') return (WORD)c;
    }
    if (strcmp(name, "space") == 0) return VK_SPACE;
    if (strcmp(name, "return") == 0) return VK_RETURN;
    if (strcmp(name, "escape") == 0) return VK_ESCAPE;
    if (strcmp(name, "up") == 0) return VK_UP;
    if (strcmp(name, "down") == 0) return VK_DOWN;
    if (strcmp(name, "left") == 0) return VK_LEFT;
    if (strcmp(name, "right") == 0) return VK_RIGHT;
    return 0;
}

int main(int argc, char **argv) {
    join_game_desktop();
    if (argc < 2) {
        fprintf(stderr, "usage: %s <move|click|down|up|key|type> ...\n", argv[0]);
        return 2;
    }
    if (strcmp(argv[1], "move") == 0 && argc >= 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
        printf("moved %s %s\n", argv[2], argv[3]);
        return 0;
    }
    if (strcmp(argv[1], "down") == 0 && argc >= 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
        left_down();
        printf("down %s %s\n", argv[2], argv[3]);
        return 0;
    }
    if (strcmp(argv[1], "up") == 0 && argc >= 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
        left_up();
        printf("up %s %s\n", argv[2], argv[3]);
        return 0;
    }
    if (strcmp(argv[1], "click") == 0 && argc >= 4) {
        move_to(atoi(argv[2]), atoi(argv[3]));
        Sleep(20);
        left_down();
        Sleep(150);
        left_up();
        printf("click %s %s\n", argv[2], argv[3]);
        return 0;
    }
    if (strcmp(argv[1], "key") == 0 && argc >= 4) {
        WORD vk = vk_for_name(argv[2]);
        if (!vk) {
            fprintf(stderr, "unknown key %s\n", argv[2]);
            return 2;
        }
        INPUT inp;
        ZeroMemory(&inp, sizeof(inp));
        inp.type = INPUT_KEYBOARD;
        inp.ki.wVk = vk;
        if (strcmp(argv[3], "down") == 0) {
            SendInput(1, &inp, sizeof(inp));
        } else {
            inp.ki.dwFlags = KEYEVENTF_KEYUP;
            SendInput(1, &inp, sizeof(inp));
        }
        printf("key %s %s\n", argv[2], argv[3]);
        return 0;
    }
    if (strcmp(argv[1], "type") == 0 && argc >= 3) {
        const char *text = argv[2];
        for (size_t i = 0; text[i] != '\0'; i++) {
            INPUT inp[2];
            ZeroMemory(inp, sizeof(inp));
            inp[0].type = INPUT_KEYBOARD;
            inp[0].ki.dwFlags = KEYEVENTF_UNICODE;
            inp[0].ki.wScan = (WORD)(unsigned char)text[i];
            inp[1] = inp[0];
            inp[1].ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
            SendInput(2, inp, sizeof(INPUT));
            Sleep(20);
        }
        printf("typed %s\n", text);
        return 0;
    }
    fprintf(stderr, "unrecognised command\n");
    return 2;
}
