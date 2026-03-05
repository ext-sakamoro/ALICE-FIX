/*
    ALICE-FIX Unity C# Bindings
    Copyright (C) 2026 Moroya Sakamoto

    33 DllImport + 5 RAII IDisposable handles (MessageHandle, BuilderHandle,
    SessionHandle, StringHandle, BytesHandle).
*/

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace AliceFix
{
    // -----------------------------------------------------------------------
    // Native imports — 33 extern "C" functions
    // -----------------------------------------------------------------------

    internal static class Native
    {
        private const string Lib = "alice_fix";

        // Memory management
        [DllImport(Lib)] internal static extern void af_fix_string_free(IntPtr s);
        [DllImport(Lib)] internal static extern void af_fix_bytes_free(IntPtr ptr, int len);

        // FixMessage
        [DllImport(Lib)] internal static extern IntPtr af_fix_message_new(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string beginString,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string msgType);
        [DllImport(Lib)] internal static extern void af_fix_message_free(IntPtr msg);
        [DllImport(Lib)] internal static extern void af_fix_message_set(IntPtr msg, uint tag,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string value);
        [DllImport(Lib)] internal static extern IntPtr af_fix_message_get(IntPtr msg, uint tag);
        [DllImport(Lib)] internal static extern byte af_fix_message_get_i64(IntPtr msg, uint tag, out long value);
        [DllImport(Lib)] internal static extern byte af_fix_message_get_u64(IntPtr msg, uint tag, out ulong value);
        [DllImport(Lib)] internal static extern IntPtr af_fix_message_begin_string(IntPtr msg);
        [DllImport(Lib)] internal static extern IntPtr af_fix_message_msg_type(IntPtr msg);

        // FixBuilder
        [DllImport(Lib)] internal static extern IntPtr af_fix_builder_new(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string beginString,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string msgType);
        [DllImport(Lib)] internal static extern void af_fix_builder_free(IntPtr builder);
        [DllImport(Lib)] internal static extern void af_fix_builder_field(IntPtr builder, uint tag,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string value);
        [DllImport(Lib)] internal static extern void af_fix_builder_field_i64(IntPtr builder, uint tag, long value);
        [DllImport(Lib)] internal static extern void af_fix_builder_field_u64(IntPtr builder, uint tag, ulong value);
        [DllImport(Lib)] internal static extern IntPtr af_fix_builder_build(IntPtr builder, out int outLen);

        // Parser
        [DllImport(Lib)] internal static extern IntPtr af_fix_parse(IntPtr input, int len);
        [DllImport(Lib)] internal static extern byte af_fix_checksum(IntPtr bytes, int len);

        // FixSession
        [DllImport(Lib)] internal static extern IntPtr af_fix_session_new(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string sender,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string target,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string beginString);
        [DllImport(Lib)] internal static extern void af_fix_session_free(IntPtr session);
        [DllImport(Lib)] internal static extern byte af_fix_session_state(IntPtr session);
        [DllImport(Lib)] internal static extern ulong af_fix_session_next_outgoing_seq(IntPtr session);
        [DllImport(Lib)] internal static extern byte af_fix_session_validate_incoming_seq(IntPtr session, ulong seq);
        [DllImport(Lib)] internal static extern IntPtr af_fix_session_build_logon(IntPtr session, out int outLen);
        [DllImport(Lib)] internal static extern IntPtr af_fix_session_build_logout(IntPtr session, out int outLen);
        [DllImport(Lib)] internal static extern IntPtr af_fix_session_build_heartbeat(IntPtr session, out int outLen);

        // Convert
        [DllImport(Lib)] internal static extern IntPtr af_fix_side_to_fix(byte side);
        [DllImport(Lib)] internal static extern sbyte af_fix_side_from_fix(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string fixSide);
        [DllImport(Lib)] internal static extern IntPtr af_fix_ord_type_to_fix(byte ordType);
        [DllImport(Lib)] internal static extern sbyte af_fix_ord_type_from_fix(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string fixType);
        [DllImport(Lib)] internal static extern IntPtr af_fix_tif_to_fix(byte tif);
        [DllImport(Lib)] internal static extern sbyte af_fix_tif_from_fix(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string fixTif);

        // Version
        [DllImport(Lib)] internal static extern IntPtr af_fix_version();
    }

    // -----------------------------------------------------------------------
    // RAII handles
    // -----------------------------------------------------------------------

    /// Owned FIX message with O(1) tag lookup.
    public class MessageHandle : IDisposable
    {
        internal IntPtr Ptr;
        public MessageHandle(IntPtr ptr) { Ptr = ptr; }

        public MessageHandle(string beginString, string msgType)
        {
            Ptr = Native.af_fix_message_new(beginString, msgType);
        }

        public void Set(uint tag, string value) => Native.af_fix_message_set(Ptr, tag, value);

        public string Get(uint tag)
        {
            IntPtr s = Native.af_fix_message_get(Ptr, tag);
            if (s == IntPtr.Zero) return null;
            string result = Marshal.PtrToStringUTF8(s);
            Native.af_fix_string_free(s);
            return result;
        }

        public long? GetI64(uint tag)
        {
            if (Native.af_fix_message_get_i64(Ptr, tag, out long val) != 0) return val;
            return null;
        }

        public ulong? GetU64(uint tag)
        {
            if (Native.af_fix_message_get_u64(Ptr, tag, out ulong val) != 0) return val;
            return null;
        }

        public string BeginString
        {
            get
            {
                IntPtr s = Native.af_fix_message_begin_string(Ptr);
                if (s == IntPtr.Zero) return null;
                string result = Marshal.PtrToStringUTF8(s);
                Native.af_fix_string_free(s);
                return result;
            }
        }

        public string MsgType
        {
            get
            {
                IntPtr s = Native.af_fix_message_msg_type(Ptr);
                if (s == IntPtr.Zero) return null;
                string result = Marshal.PtrToStringUTF8(s);
                Native.af_fix_string_free(s);
                return result;
            }
        }

        public void Dispose()
        {
            if (Ptr != IntPtr.Zero) { Native.af_fix_message_free(Ptr); Ptr = IntPtr.Zero; }
        }
    }

    /// FIX message builder with auto BodyLength/Checksum.
    public class BuilderHandle : IDisposable
    {
        internal IntPtr Ptr;

        public BuilderHandle(string beginString, string msgType)
        {
            Ptr = Native.af_fix_builder_new(beginString, msgType);
        }

        public BuilderHandle Field(uint tag, string value)   { Native.af_fix_builder_field(Ptr, tag, value);     return this; }
        public BuilderHandle FieldI64(uint tag, long value)   { Native.af_fix_builder_field_i64(Ptr, tag, value); return this; }
        public BuilderHandle FieldU64(uint tag, ulong value)  { Native.af_fix_builder_field_u64(Ptr, tag, value); return this; }

        public BytesHandle Build()
        {
            IntPtr ptr = Native.af_fix_builder_build(Ptr, out int len);
            return new BytesHandle(ptr, len);
        }

        public void Dispose()
        {
            if (Ptr != IntPtr.Zero) { Native.af_fix_builder_free(Ptr); Ptr = IntPtr.Zero; }
        }
    }

    /// FIX session state machine with sequence tracking.
    public class SessionHandle : IDisposable
    {
        internal IntPtr Ptr;

        public SessionHandle(string sender, string target, string beginString)
        {
            Ptr = Native.af_fix_session_new(sender, target, beginString);
        }

        public byte State => Native.af_fix_session_state(Ptr);
        public ulong NextOutgoingSeq() => Native.af_fix_session_next_outgoing_seq(Ptr);
        public bool ValidateIncomingSeq(ulong seq) => Native.af_fix_session_validate_incoming_seq(Ptr, seq) != 0;

        public BytesHandle BuildLogon()     { return MakeBytes(Native.af_fix_session_build_logon(Ptr, out int len), len); }
        public BytesHandle BuildLogout()    { return MakeBytes(Native.af_fix_session_build_logout(Ptr, out int len), len); }
        public BytesHandle BuildHeartbeat() { return MakeBytes(Native.af_fix_session_build_heartbeat(Ptr, out int len), len); }

        private static BytesHandle MakeBytes(IntPtr ptr, int len) => new BytesHandle(ptr, len);

        public void Dispose()
        {
            if (Ptr != IntPtr.Zero) { Native.af_fix_session_free(Ptr); Ptr = IntPtr.Zero; }
        }
    }

    /// Owned string returned by ALICE-FIX. Freed on Dispose.
    public class StringHandle : IDisposable
    {
        internal IntPtr Ptr;
        public StringHandle(IntPtr ptr) { Ptr = ptr; }
        public override string ToString() => Ptr != IntPtr.Zero ? Marshal.PtrToStringUTF8(Ptr) : null;
        public void Dispose()
        {
            if (Ptr != IntPtr.Zero) { Native.af_fix_string_free(Ptr); Ptr = IntPtr.Zero; }
        }
    }

    /// Owned byte buffer returned by build functions. Freed on Dispose.
    public class BytesHandle : IDisposable
    {
        internal IntPtr Ptr;
        internal int Length;
        public BytesHandle(IntPtr ptr, int len) { Ptr = ptr; Length = len; }

        public byte[] ToArray()
        {
            if (Ptr == IntPtr.Zero || Length <= 0) return Array.Empty<byte>();
            byte[] arr = new byte[Length];
            Marshal.Copy(Ptr, arr, 0, Length);
            return arr;
        }

        public void Dispose()
        {
            if (Ptr != IntPtr.Zero) { Native.af_fix_bytes_free(Ptr, Length); Ptr = IntPtr.Zero; }
        }
    }

    // -----------------------------------------------------------------------
    // Static helpers
    // -----------------------------------------------------------------------

    /// Parser and conversion utilities.
    public static class AliceFix
    {
        public static MessageHandle Parse(byte[] data)
        {
            if (data == null || data.Length == 0) return null;
            unsafe
            {
                fixed (byte* p = data)
                {
                    IntPtr msg = Native.af_fix_parse((IntPtr)p, data.Length);
                    return msg != IntPtr.Zero ? new MessageHandle(msg) : null;
                }
            }
        }

        public static byte Checksum(byte[] data)
        {
            if (data == null || data.Length == 0) return 0;
            unsafe
            {
                fixed (byte* p = data)
                {
                    return Native.af_fix_checksum((IntPtr)p, data.Length);
                }
            }
        }

        public static string SideToFix(byte side)
        {
            IntPtr s = Native.af_fix_side_to_fix(side);
            return s != IntPtr.Zero ? Marshal.PtrToStringUTF8(s) : null;
        }

        public static sbyte SideFromFix(string fixSide) => Native.af_fix_side_from_fix(fixSide);

        public static string OrdTypeToFix(byte ordType)
        {
            IntPtr s = Native.af_fix_ord_type_to_fix(ordType);
            return s != IntPtr.Zero ? Marshal.PtrToStringUTF8(s) : null;
        }

        public static sbyte OrdTypeFromFix(string fixType) => Native.af_fix_ord_type_from_fix(fixType);

        public static string TifToFix(byte tif)
        {
            IntPtr s = Native.af_fix_tif_to_fix(tif);
            return s != IntPtr.Zero ? Marshal.PtrToStringUTF8(s) : null;
        }

        public static sbyte TifFromFix(string fixTif) => Native.af_fix_tif_from_fix(fixTif);

        public static string Version => Marshal.PtrToStringUTF8(Native.af_fix_version());
    }

    // -----------------------------------------------------------------------
    // FIX tag constants
    // -----------------------------------------------------------------------

    public static class Tag
    {
        public const uint BeginString  = 8;
        public const uint BodyLength   = 9;
        public const uint MsgType      = 35;
        public const uint SenderCompId = 49;
        public const uint TargetCompId = 56;
        public const uint MsgSeqNum    = 34;
        public const uint SendingTime  = 52;
        public const uint CheckSum     = 10;
        public const uint ClOrdId      = 11;
        public const uint OrderId      = 37;
        public const uint ExecId       = 17;
        public const uint Symbol       = 55;
        public const uint Side         = 54;
        public const uint OrdType      = 40;
        public const uint Price        = 44;
        public const uint OrderQty     = 38;
        public const uint TimeInForce  = 59;
        public const uint ExecType     = 150;
        public const uint OrdStatus    = 39;
        public const uint LastPx       = 31;
        public const uint LastQty      = 32;
        public const uint LeavesQty    = 151;
        public const uint CumQty       = 14;
        public const uint AvgPx        = 6;
        public const uint TransactTime = 60;
        public const uint Text         = 58;
    }
}
