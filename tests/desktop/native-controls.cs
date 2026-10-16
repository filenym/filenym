using System;
using System.Runtime.InteropServices;
using System.Text;

// Native picker controls sometimes expose neither InvokePattern nor ValuePattern.
// Every message is bounded and checked against the explicitly selected app PID.
public static class FileNymNativeControl
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetClassName(IntPtr handle, StringBuilder text, int count);
    [DllImport("user32.dll")]
    static extern uint GetWindowThreadProcessId(IntPtr handle, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr SendMessageTimeout(IntPtr handle, uint message, IntPtr wParam,
        string lParam, uint flags, uint timeout, out UIntPtr result);
    [DllImport("user32.dll")]
    static extern bool EnumChildWindows(IntPtr handle, EnumWindow callback, IntPtr state);
    delegate bool EnumWindow(IntPtr handle, IntPtr state);

    static void RequireOwner(IntPtr handle, int expected)
    {
        uint owner;
        GetWindowThreadProcessId(handle, out owner);
        if (handle == IntPtr.Zero || owner != (uint)expected)
            throw new Exception("Not an owned native control");
    }

    public static string Class(IntPtr handle)
    {
        var text = new StringBuilder(256);
        GetClassName(handle, text, 256);
        return text.ToString();
    }

    public static void SetText(IntPtr handle, int expected, string value)
    {
        RequireOwner(handle, expected);
        IntPtr edit = Class(handle) == "Edit" ? handle : IntPtr.Zero;
        int count = 0;
        if (edit == IntPtr.Zero)
        {
            EnumChildWindows(handle, (child, state) => {
                if (Class(child) == "Edit") { edit = child; count++; }
                return true;
            }, IntPtr.Zero);
            if (count != 1) throw new Exception("Expected one native edit");
        }
        RequireOwner(edit, expected);
        UIntPtr result;
        if (SendMessageTimeout(edit, 0x000C, IntPtr.Zero, value, 2, 3000, out result) == IntPtr.Zero)
            throw new Exception("Native edit did not acknowledge");
    }

    public static void Click(IntPtr handle, int expected)
    {
        RequireOwner(handle, expected);
        if (Class(handle) != "Button") throw new Exception("Expected a native button");
        UIntPtr result;
        if (SendMessageTimeout(handle, 0x00F5, IntPtr.Zero, null, 2, 3000, out result) == IntPtr.Zero)
            throw new Exception("Native button did not acknowledge");
    }
}
