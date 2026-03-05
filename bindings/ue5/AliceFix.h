/*
    ALICE-FIX  UE5 C++ Bindings
    Copyright (C) 2026 Moroya Sakamoto

    33 extern "C" functions + 5 RAII unique_ptr handles
    (MessagePtr, BuilderPtr, SessionPtr, StringPtr, BytesPtr).
*/

#pragma once

#include <cstdint>
#include <cstring>
#include <memory>
#include <string>
#include <optional>
#include <vector>

// -----------------------------------------------------------------------
// C-ABI declarations — 33 functions
// -----------------------------------------------------------------------

extern "C"
{
    // Memory management
    void af_fix_string_free(char* s);
    void af_fix_bytes_free(uint8_t* ptr, int32_t len);

    // FixMessage
    void* af_fix_message_new(const char* begin_string, const char* msg_type);
    void  af_fix_message_free(void* msg);
    void  af_fix_message_set(void* msg, uint32_t tag, const char* value);
    char* af_fix_message_get(const void* msg, uint32_t tag);
    uint8_t af_fix_message_get_i64(const void* msg, uint32_t tag, int64_t* out);
    uint8_t af_fix_message_get_u64(const void* msg, uint32_t tag, uint64_t* out);
    char* af_fix_message_begin_string(const void* msg);
    char* af_fix_message_msg_type(const void* msg);

    // FixBuilder
    void* af_fix_builder_new(const char* begin_string, const char* msg_type);
    void  af_fix_builder_free(void* builder);
    void  af_fix_builder_field(void* builder, uint32_t tag, const char* value);
    void  af_fix_builder_field_i64(void* builder, uint32_t tag, int64_t value);
    void  af_fix_builder_field_u64(void* builder, uint32_t tag, uint64_t value);
    uint8_t* af_fix_builder_build(const void* builder, int32_t* out_len);

    // Parser
    void*   af_fix_parse(const uint8_t* input, int32_t len);
    uint8_t af_fix_checksum(const uint8_t* bytes, int32_t len);

    // FixSession
    void* af_fix_session_new(const char* sender, const char* target, const char* begin_string);
    void  af_fix_session_free(void* session);
    uint8_t  af_fix_session_state(const void* session);
    uint64_t af_fix_session_next_outgoing_seq(void* session);
    uint8_t  af_fix_session_validate_incoming_seq(void* session, uint64_t seq);
    uint8_t* af_fix_session_build_logon(void* session, int32_t* out_len);
    uint8_t* af_fix_session_build_logout(void* session, int32_t* out_len);
    uint8_t* af_fix_session_build_heartbeat(void* session, int32_t* out_len);

    // Convert
    const char* af_fix_side_to_fix(uint8_t side);
    int8_t af_fix_side_from_fix(const char* fix_side);
    const char* af_fix_ord_type_to_fix(uint8_t ord_type);
    int8_t af_fix_ord_type_from_fix(const char* fix_type);
    const char* af_fix_tif_to_fix(uint8_t tif);
    int8_t af_fix_tif_from_fix(const char* fix_tif);

    // Version
    const char* af_fix_version();
}

// -----------------------------------------------------------------------
// FIX tag constants
// -----------------------------------------------------------------------

namespace AliceFix
{
    namespace Tag
    {
        constexpr uint32_t BeginString  = 8;
        constexpr uint32_t BodyLength   = 9;
        constexpr uint32_t MsgType      = 35;
        constexpr uint32_t SenderCompId = 49;
        constexpr uint32_t TargetCompId = 56;
        constexpr uint32_t MsgSeqNum    = 34;
        constexpr uint32_t SendingTime  = 52;
        constexpr uint32_t CheckSum     = 10;
        constexpr uint32_t ClOrdId      = 11;
        constexpr uint32_t OrderId      = 37;
        constexpr uint32_t ExecId       = 17;
        constexpr uint32_t Symbol       = 55;
        constexpr uint32_t Side         = 54;
        constexpr uint32_t OrdType      = 40;
        constexpr uint32_t Price        = 44;
        constexpr uint32_t OrderQty     = 38;
        constexpr uint32_t TimeInForce  = 59;
        constexpr uint32_t ExecType     = 150;
        constexpr uint32_t OrdStatus    = 39;
        constexpr uint32_t LastPx       = 31;
        constexpr uint32_t LastQty      = 32;
        constexpr uint32_t LeavesQty    = 151;
        constexpr uint32_t CumQty       = 14;
        constexpr uint32_t AvgPx        = 6;
        constexpr uint32_t TransactTime = 60;
        constexpr uint32_t Text         = 58;
    }

    // -----------------------------------------------------------------------
    // RAII wrappers
    // -----------------------------------------------------------------------

    /// Owned string returned by ALICE-FIX (freed on destruction).
    struct StringPtr
    {
        char* Raw = nullptr;
        StringPtr() = default;
        explicit StringPtr(char* p) : Raw(p) {}
        ~StringPtr() { if (Raw) af_fix_string_free(Raw); }
        StringPtr(StringPtr&& o) noexcept : Raw(o.Raw) { o.Raw = nullptr; }
        StringPtr& operator=(StringPtr&& o) noexcept { std::swap(Raw, o.Raw); return *this; }
        StringPtr(const StringPtr&) = delete;
        StringPtr& operator=(const StringPtr&) = delete;
        explicit operator bool() const { return Raw != nullptr; }
        std::string Str() const { return Raw ? std::string(Raw) : std::string(); }
    };

    /// Owned byte buffer returned by build functions (freed on destruction).
    struct BytesPtr
    {
        uint8_t* Raw = nullptr;
        int32_t  Len = 0;
        BytesPtr() = default;
        BytesPtr(uint8_t* p, int32_t l) : Raw(p), Len(l) {}
        ~BytesPtr() { if (Raw) af_fix_bytes_free(Raw, Len); }
        BytesPtr(BytesPtr&& o) noexcept : Raw(o.Raw), Len(o.Len) { o.Raw = nullptr; o.Len = 0; }
        BytesPtr& operator=(BytesPtr&& o) noexcept { std::swap(Raw, o.Raw); std::swap(Len, o.Len); return *this; }
        BytesPtr(const BytesPtr&) = delete;
        BytesPtr& operator=(const BytesPtr&) = delete;
        explicit operator bool() const { return Raw != nullptr && Len > 0; }
        std::vector<uint8_t> ToVector() const { return Raw ? std::vector<uint8_t>(Raw, Raw + Len) : std::vector<uint8_t>(); }
    };

    // -----------------------------------------------------------------------
    // MessagePtr — parsed FIX message
    // -----------------------------------------------------------------------

    struct MessageDeleter { void operator()(void* p) const { af_fix_message_free(p); } };
    using MessagePtr = std::unique_ptr<void, MessageDeleter>;

    inline MessagePtr MakeMessage(const char* begin_string, const char* msg_type)
    {
        return MessagePtr(af_fix_message_new(begin_string, msg_type));
    }

    inline void SetField(const MessagePtr& m, uint32_t tag, const char* value)
    {
        af_fix_message_set(m.get(), tag, value);
    }

    inline StringPtr GetField(const MessagePtr& m, uint32_t tag)
    {
        return StringPtr(af_fix_message_get(m.get(), tag));
    }

    inline std::optional<int64_t> GetI64(const MessagePtr& m, uint32_t tag)
    {
        int64_t v = 0;
        if (af_fix_message_get_i64(m.get(), tag, &v)) return v;
        return std::nullopt;
    }

    inline std::optional<uint64_t> GetU64(const MessagePtr& m, uint32_t tag)
    {
        uint64_t v = 0;
        if (af_fix_message_get_u64(m.get(), tag, &v)) return v;
        return std::nullopt;
    }

    inline StringPtr GetBeginString(const MessagePtr& m)
    {
        return StringPtr(af_fix_message_begin_string(m.get()));
    }

    inline StringPtr GetMsgType(const MessagePtr& m)
    {
        return StringPtr(af_fix_message_msg_type(m.get()));
    }

    // -----------------------------------------------------------------------
    // BuilderPtr — FIX message serializer
    // -----------------------------------------------------------------------

    struct BuilderDeleter { void operator()(void* p) const { af_fix_builder_free(p); } };
    using BuilderPtr = std::unique_ptr<void, BuilderDeleter>;

    inline BuilderPtr MakeBuilder(const char* begin_string, const char* msg_type)
    {
        return BuilderPtr(af_fix_builder_new(begin_string, msg_type));
    }

    inline void AddField(const BuilderPtr& b, uint32_t tag, const char* value)
    {
        af_fix_builder_field(b.get(), tag, value);
    }

    inline void AddFieldI64(const BuilderPtr& b, uint32_t tag, int64_t value)
    {
        af_fix_builder_field_i64(b.get(), tag, value);
    }

    inline void AddFieldU64(const BuilderPtr& b, uint32_t tag, uint64_t value)
    {
        af_fix_builder_field_u64(b.get(), tag, value);
    }

    inline BytesPtr Build(const BuilderPtr& b)
    {
        int32_t len = 0;
        uint8_t* ptr = af_fix_builder_build(b.get(), &len);
        return BytesPtr(ptr, len);
    }

    // -----------------------------------------------------------------------
    // Parser helpers
    // -----------------------------------------------------------------------

    inline MessagePtr Parse(const uint8_t* data, int32_t len)
    {
        return MessagePtr(af_fix_parse(data, len));
    }

    inline uint8_t Checksum(const uint8_t* data, int32_t len)
    {
        return af_fix_checksum(data, len);
    }

    // -----------------------------------------------------------------------
    // SessionPtr — FIX session state machine
    // -----------------------------------------------------------------------

    struct SessionDeleter { void operator()(void* p) const { af_fix_session_free(p); } };
    using SessionPtr = std::unique_ptr<void, SessionDeleter>;

    inline SessionPtr MakeSession(const char* sender, const char* target, const char* begin_string)
    {
        return SessionPtr(af_fix_session_new(sender, target, begin_string));
    }

    inline uint8_t GetState(const SessionPtr& s) { return af_fix_session_state(s.get()); }

    inline uint64_t NextOutgoingSeq(const SessionPtr& s)
    {
        return af_fix_session_next_outgoing_seq(s.get());
    }

    inline bool ValidateIncomingSeq(const SessionPtr& s, uint64_t seq)
    {
        return af_fix_session_validate_incoming_seq(s.get(), seq) != 0;
    }

    inline BytesPtr BuildLogon(const SessionPtr& s)
    {
        int32_t len = 0;
        uint8_t* ptr = af_fix_session_build_logon(s.get(), &len);
        return BytesPtr(ptr, len);
    }

    inline BytesPtr BuildLogout(const SessionPtr& s)
    {
        int32_t len = 0;
        uint8_t* ptr = af_fix_session_build_logout(s.get(), &len);
        return BytesPtr(ptr, len);
    }

    inline BytesPtr BuildHeartbeat(const SessionPtr& s)
    {
        int32_t len = 0;
        uint8_t* ptr = af_fix_session_build_heartbeat(s.get(), &len);
        return BytesPtr(ptr, len);
    }

    // -----------------------------------------------------------------------
    // Convert helpers
    // -----------------------------------------------------------------------

    inline const char* SideToFix(uint8_t side) { return af_fix_side_to_fix(side); }
    inline int8_t SideFromFix(const char* s)   { return af_fix_side_from_fix(s); }

    inline const char* OrdTypeToFix(uint8_t t) { return af_fix_ord_type_to_fix(t); }
    inline int8_t OrdTypeFromFix(const char* s){ return af_fix_ord_type_from_fix(s); }

    inline const char* TifToFix(uint8_t t)     { return af_fix_tif_to_fix(t); }
    inline int8_t TifFromFix(const char* s)    { return af_fix_tif_from_fix(s); }

    // -----------------------------------------------------------------------
    // Version
    // -----------------------------------------------------------------------

    inline const char* Version() { return af_fix_version(); }

} // namespace AliceFix
