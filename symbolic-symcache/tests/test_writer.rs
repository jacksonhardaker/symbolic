use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{borrow::Cow, convert::Infallible};

use symbolic_common::{Arch, ByteView, CodeId, DebugId, Name};
use symbolic_debuginfo::{
    sourcebundle::SourceFileDescriptor, DebugSession, FileEntry, FileFormat, Function, LineInfo,
    Object, ObjectKind, ObjectLike, Symbol, SymbolMap,
};
use symbolic_symcache::{FunctionsDebug, SymCache, SymCacheConverter, SymCacheConverterOptions};
use symbolic_testutils::fixture;

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Copy, Debug, Default)]
struct MockDebugSession;

impl<'session> DebugSession<'session> for MockDebugSession {
    type Error = Infallible;
    type FunctionIterator = std::iter::Empty<Result<Function<'session>, Self::Error>>;
    type FileIterator = std::iter::Empty<Result<FileEntry<'session>, Self::Error>>;

    fn functions(&'session self) -> Self::FunctionIterator {
        std::iter::empty()
    }

    fn files(&'session self) -> Self::FileIterator {
        std::iter::empty()
    }

    fn source_by_path(&self, _path: &str) -> Result<Option<SourceFileDescriptor<'_>>, Self::Error> {
        Ok(None)
    }

    fn line_records(
        &'session self,
        address: u64,
        size: u64,
    ) -> Result<Vec<LineInfo<'session>>, Self::Error> {
        let query_end = address.saturating_add(size.max(1));

        Ok(vec![
            LineInfo {
                address,
                size: Some(0x10),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 405,
            },
            LineInfo {
                address: address + 0x10,
                size: Some(query_end.saturating_sub(address + 0x10)),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 407,
            },
        ])
    }
}

#[derive(Clone, Debug, Default)]
struct MockObject {
    symbols: Vec<Symbol<'static>>,
}

impl MockObject {
    fn new(symbols: Vec<Symbol<'static>>) -> Self {
        Self { symbols }
    }
}

#[derive(Clone, Debug)]
struct CountingDebugSession {
    functions: Vec<Function<'static>>,
    line_record_calls: Arc<AtomicUsize>,
    line_records: Vec<LineInfo<'static>>,
}

impl<'session> DebugSession<'session> for CountingDebugSession {
    type Error = Infallible;
    type FunctionIterator = std::vec::IntoIter<Result<Function<'session>, Self::Error>>;
    type FileIterator = std::iter::Empty<Result<FileEntry<'session>, Self::Error>>;

    fn functions(&'session self) -> Self::FunctionIterator {
        self.functions
            .clone()
            .into_iter()
            .map(Ok)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn files(&'session self) -> Self::FileIterator {
        std::iter::empty()
    }

    fn source_by_path(&self, _path: &str) -> Result<Option<SourceFileDescriptor<'_>>, Self::Error> {
        Ok(None)
    }

    fn line_records(
        &'session self,
        _address: u64,
        _size: u64,
    ) -> Result<Vec<LineInfo<'session>>, Self::Error> {
        self.line_record_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.line_records.clone())
    }
}

#[derive(Clone, Debug)]
struct FunctionBackedMockObject {
    symbols: Vec<Symbol<'static>>,
    functions: Vec<Function<'static>>,
    line_record_calls: Arc<AtomicUsize>,
    line_records: Vec<LineInfo<'static>>,
}

impl FunctionBackedMockObject {
    fn new(
        symbols: Vec<Symbol<'static>>,
        functions: Vec<Function<'static>>,
        line_record_calls: Arc<AtomicUsize>,
        line_records: Vec<LineInfo<'static>>,
    ) -> Self {
        Self {
            symbols,
            functions,
            line_record_calls,
            line_records,
        }
    }
}

impl<'object> ObjectLike<'static, 'object> for MockObject {
    type Error = Infallible;
    type Session = MockDebugSession;
    type SymbolIterator = std::vec::IntoIter<Symbol<'static>>;

    fn file_format(&self) -> FileFormat {
        FileFormat::MachO
    }

    fn code_id(&self) -> Option<CodeId> {
        None
    }

    fn debug_id(&self) -> DebugId {
        DebugId::default()
    }

    fn arch(&self) -> Arch {
        Arch::Arm64
    }

    fn kind(&self) -> ObjectKind {
        ObjectKind::Debug
    }

    fn load_address(&self) -> u64 {
        0
    }

    fn has_symbols(&self) -> bool {
        true
    }

    fn symbols(&'object self) -> Self::SymbolIterator {
        self.symbols.clone().into_iter()
    }

    fn symbol_map(&self) -> SymbolMap<'static> {
        self.symbols.clone().into_iter().collect()
    }

    fn has_debug_info(&self) -> bool {
        true
    }

    fn debug_session(&'object self) -> Result<Self::Session, Self::Error> {
        Ok(MockDebugSession)
    }

    fn has_unwind_info(&self) -> bool {
        false
    }

    fn has_sources(&self) -> bool {
        false
    }

    fn is_malformed(&self) -> bool {
        false
    }
}

impl<'object> ObjectLike<'static, 'object> for FunctionBackedMockObject {
    type Error = Infallible;
    type Session = CountingDebugSession;
    type SymbolIterator = std::vec::IntoIter<Symbol<'static>>;

    fn file_format(&self) -> FileFormat {
        FileFormat::MachO
    }

    fn code_id(&self) -> Option<CodeId> {
        None
    }

    fn debug_id(&self) -> DebugId {
        DebugId::default()
    }

    fn arch(&self) -> Arch {
        Arch::Arm64
    }

    fn kind(&self) -> ObjectKind {
        ObjectKind::Debug
    }

    fn load_address(&self) -> u64 {
        0
    }

    fn has_symbols(&self) -> bool {
        true
    }

    fn symbols(&'object self) -> Self::SymbolIterator {
        self.symbols.clone().into_iter()
    }

    fn symbol_map(&self) -> SymbolMap<'static> {
        self.symbols.clone().into_iter().collect()
    }

    fn has_debug_info(&self) -> bool {
        true
    }

    fn debug_session(&'object self) -> Result<Self::Session, Self::Error> {
        Ok(CountingDebugSession {
            functions: self.functions.clone(),
            line_record_calls: self.line_record_calls.clone(),
            line_records: self.line_records.clone(),
        })
    }

    fn has_unwind_info(&self) -> bool {
        false
    }

    fn has_sources(&self) -> bool {
        false
    }

    fn is_malformed(&self) -> bool {
        false
    }
}

#[test]
fn test_write_header_linux() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("linux/crash.debug"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;

    #[cfg(target_endian = "little")]
    {
        assert!(buffer.starts_with(b"SYMC"));
    }

    let symcache = SymCache::parse(&buffer)?;
    insta::assert_debug_snapshot!(symcache, @r#"
    SymCache {
        version: 9,
        debug_id: DebugId {
            uuid: "c0bcc3f1-9827-fe65-3058-404b2831d9e6",
            appendix: 0,
        },
        arch: Amd64,
        files: 55,
        functions: 697,
        source_locations: 8431,
        ranges: 6975,
        string_bytes: 49877,
    }
    "#);

    Ok(())
}

#[test]
fn test_write_functions_linux() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("linux/crash.debug"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    insta::assert_debug_snapshot!("functions_linux", FunctionsDebug(&symcache));

    Ok(())
}

#[test]
fn test_write_header_macos() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("macos/crash.dSYM/Contents/Resources/DWARF/crash"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    insta::assert_debug_snapshot!(symcache, @r#"
    SymCache {
        version: 9,
        debug_id: DebugId {
            uuid: "67e9247c-814e-392b-a027-dbde6748fcbf",
            appendix: 0,
        },
        arch: Amd64,
        files: 36,
        functions: 639,
        source_locations: 7382,
        ranges: 5965,
        string_bytes: 40958,
    }
    "#);

    Ok(())
}

#[test]
fn test_write_functions_macos() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("macos/crash.dSYM/Contents/Resources/DWARF/crash"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    insta::assert_debug_snapshot!("functions_macos", FunctionsDebug(&symcache));

    Ok(())
}

// Tests that functions with identical names, compilation directories, and languages but different
// entry_pcs have separate, distinct entries in the symcache. The specific use case generating this
// is two identically-named static C functions nestled in two different files sharing a common
// compilation directory. See the overlapping_funcs directory in sentry-testutils for related files.
#[test]
fn test_write_functions_overlapping_funcs() -> Result<(), Error> {
    let buffer = ByteView::open(fixture(
        "macos/overlapping_funcs.dSYM/Contents/Resources/DWARF/overlapping_funcs",
    ))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    insta::assert_debug_snapshot!("overlapping_funcs", FunctionsDebug(&symcache));

    Ok(())
}

#[test]
fn test_write_large_symbol_names() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("regression/large_symbol.sym"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    SymCache::parse(&buffer)?;

    Ok(())
}

/// This tests the fix for the bug described in
/// https://github.com/getsentry/symbolic/issues/284#issue-726898083
#[test]
fn test_lookup_no_lines() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("xul.sym"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    let symbols = symcache.lookup(0xc6dd98).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    let name = symbols[0].function().name();

    assert_eq!(
        name,
        "std::_Func_impl_no_alloc<`lambda at \
        /builds/worker/checkouts/gecko/netwerk/\
        protocol/http/HttpChannelChild.cpp:411:7',void>::_Do_call()"
    );

    Ok(())
}

#[test]
fn test_symbol_fallback_recovers_line_records() -> Result<(), Error> {
    let object = MockObject::new(vec![Symbol {
        name: Some(Cow::Borrowed("swift closure")),
        address: 0x2061c,
        size: 0x18,
    }]);

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].line(), 407);
    assert_eq!(
        symbols[0].file().map(|file| file.full_path()),
        Some("CrashCatalog.swift".to_string())
    );

    Ok(())
}

#[test]
fn test_symbol_line_recovery_is_skipped_when_functions_exist() -> Result<(), Error> {
    let line_record_calls = Arc::new(AtomicUsize::new(0));
    let object = FunctionBackedMockObject::new(
        vec![Symbol {
            name: Some(Cow::Borrowed("swift closure")),
            address: 0x2061c,
            size: 0x18,
        }],
        vec![Function {
            address: 0x2061c,
            size: 0x18,
            name: Name::from("closure #1 in DispatchQueueCrash.trigger"),
            compilation_dir: b"",
            lines: vec![LineInfo {
                address: 0x2061c,
                size: Some(0x18),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 407,
            }],
            inlinees: Vec::new(),
            inline: false,
        }],
        line_record_calls.clone(),
        vec![LineInfo {
            address: 0x2061c,
            size: Some(0x18),
            file: symbolic_debuginfo::FileInfo::from_filename(b"Ignored.swift"),
            line: 999,
        }],
    );

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].line(), 407);
    assert_eq!(line_record_calls.load(Ordering::Relaxed), 0);

    Ok(())
}

#[test]
fn test_empty_function_line_recovery_can_be_disabled() -> Result<(), Error> {
    let line_record_calls = Arc::new(AtomicUsize::new(0));
    let object = FunctionBackedMockObject::new(
        vec![Symbol {
            name: Some(Cow::Borrowed("swift closure")),
            address: 0x2061c,
            size: 0x18,
        }],
        vec![Function {
            address: 0x2061c,
            size: 0x18,
            name: Name::from("closure #1 in DispatchQueueCrash.trigger"),
            compilation_dir: b"",
            lines: Vec::new(),
            inlinees: Vec::new(),
            inline: false,
        }],
        line_record_calls.clone(),
        vec![LineInfo {
            address: 0x2061c,
            size: Some(0x18),
            file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
            line: 407,
        }],
    );

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::with_options(SymCacheConverterOptions {
        enable_empty_function_line_recovery: false,
        enable_symbol_line_recovery: true,
    });
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(
        symbols[0].function().name(),
        "closure #1 in DispatchQueueCrash.trigger"
    );
    assert_eq!(symbols[0].line(), 0);
    assert!(symbols[0].file().is_none());
    assert_eq!(line_record_calls.load(Ordering::Relaxed), 0);

    Ok(())
}

#[test]
fn test_symbol_line_recovery_runs_for_uncovered_symbol_when_functions_exist() -> Result<(), Error> {
    let line_record_calls = Arc::new(AtomicUsize::new(0));
    let object = FunctionBackedMockObject::new(
        vec![Symbol {
            name: Some(Cow::Borrowed("swift closure")),
            address: 0x2061c,
            size: 0x18,
        }],
        vec![Function {
            address: 0x1000,
            size: 0x40,
            name: Name::from("some other function"),
            compilation_dir: b"",
            lines: vec![LineInfo {
                address: 0x1000,
                size: Some(0x40),
                file: symbolic_debuginfo::FileInfo::from_filename(b"Other.swift"),
                line: 12,
            }],
            inlinees: Vec::new(),
            inline: false,
        }],
        line_record_calls.clone(),
        vec![
            LineInfo {
                address: 0x2061c,
                size: Some(0x10),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 405,
            },
            LineInfo {
                address: 0x2062c,
                size: Some(0x8),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 407,
            },
        ],
    );

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].line(), 407);
    assert_eq!(
        symbols[0].file().map(|file| file.full_path()),
        Some("CrashCatalog.swift".to_string())
    );
    assert_eq!(line_record_calls.load(Ordering::Relaxed), 1);

    Ok(())
}

#[test]
fn test_symbol_line_recovery_can_be_disabled() -> Result<(), Error> {
    let line_record_calls = Arc::new(AtomicUsize::new(0));
    let object = FunctionBackedMockObject::new(
        vec![Symbol {
            name: Some(Cow::Borrowed("swift closure")),
            address: 0x2061c,
            size: 0x18,
        }],
        vec![Function {
            address: 0x1000,
            size: 0x40,
            name: Name::from("some other function"),
            compilation_dir: b"",
            lines: vec![LineInfo {
                address: 0x1000,
                size: Some(0x40),
                file: symbolic_debuginfo::FileInfo::from_filename(b"Other.swift"),
                line: 12,
            }],
            inlinees: Vec::new(),
            inline: false,
        }],
        line_record_calls.clone(),
        vec![
            LineInfo {
                address: 0x2061c,
                size: Some(0x10),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 405,
            },
            LineInfo {
                address: 0x2062c,
                size: Some(0x8),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 407,
            },
        ],
    );

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::with_options(SymCacheConverterOptions {
        enable_empty_function_line_recovery: true,
        enable_symbol_line_recovery: false,
    });
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].function().name(), "swift closure");
    assert_eq!(symbols[0].line(), 0);
    assert!(symbols[0].file().is_none());
    assert_eq!(line_record_calls.load(Ordering::Relaxed), 0);

    Ok(())
}

#[test]
fn test_symbol_line_recovery_keeps_symbol_only_mapping_when_session_returns_no_lines(
) -> Result<(), Error> {
    let line_record_calls = Arc::new(AtomicUsize::new(0));
    let object = FunctionBackedMockObject::new(
        vec![Symbol {
            name: Some(Cow::Borrowed("swift closure")),
            address: 0x2061c,
            size: 0x18,
        }],
        vec![Function {
            address: 0x1000,
            size: 0x40,
            name: Name::from("some other function"),
            compilation_dir: b"",
            lines: vec![LineInfo {
                address: 0x1000,
                size: Some(0x40),
                file: symbolic_debuginfo::FileInfo::from_filename(b"Other.swift"),
                line: 12,
            }],
            inlinees: Vec::new(),
            inline: false,
        }],
        line_record_calls.clone(),
        Vec::new(),
    );

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].function().name(), "swift closure");
    assert_eq!(symbols[0].line(), 0);
    assert!(symbols[0].file().is_none());
    assert_eq!(line_record_calls.load(Ordering::Relaxed), 1);

    Ok(())
}

#[test]
fn test_symbol_line_recovery_overwrites_existing_unknown_source_mapping() -> Result<(), Error> {
    let line_record_calls = Arc::new(AtomicUsize::new(0));
    let object = FunctionBackedMockObject::new(
        vec![Symbol {
            name: Some(Cow::Borrowed("swift closure")),
            address: 0x2061c,
            size: 0x18,
        }],
        vec![Function {
            address: 0x2061c,
            size: 0x18,
            name: Name::from("closure #1 in DispatchQueueCrash.trigger"),
            compilation_dir: b"",
            lines: Vec::new(),
            inlinees: Vec::new(),
            inline: false,
        }],
        line_record_calls.clone(),
        vec![
            LineInfo {
                address: 0x2061c,
                size: Some(0x10),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 405,
            },
            LineInfo {
                address: 0x2062c,
                size: Some(0x8),
                file: symbolic_debuginfo::FileInfo::from_filename(b"CrashCatalog.swift"),
                line: 407,
            },
        ],
    );

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x20630).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].line(), 407);
    assert_eq!(
        symbols[0].file().map(|file| file.full_path()),
        Some("CrashCatalog.swift".to_string())
    );
    assert!(line_record_calls.load(Ordering::Relaxed) >= 1);

    Ok(())
}

/// This tests the fix for the bug described in
/// https://github.com/getsentry/symbolic/issues/284#issuecomment-715587454.
#[test]
fn test_lookup_no_size() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("libgallium_dri.sym"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    let symbols = symcache.lookup(0x1489adf).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    let name = symbols[0].function().name();

    assert_eq!(name, "nouveau_drm_screen_create");

    Ok(())
}

/// This tests the fix for the bug described in
/// https://github.com/getsentry/symbolic/issues/285.
#[test]
fn test_lookup_modulo_u16() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("xul2.sym"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;
    let symbols = symcache.lookup(0x3c105a1).collect::<Vec<_>>();

    assert_eq!(symbols.len(), 1);
    let name = symbols[0].function().name();

    assert_eq!(name, "Interpret(JSContext*, js::RunState&)");

    Ok(())
}

/// This tests the fix for the bug described in
/// https://github.com/getsentry/symbolic/issues/646.
#[test]
fn test_lookup_second_line_in_inlinee() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("macos/crash.dSYM/Contents/Resources/DWARF/crash"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    // Test an address at the second line of an inlinee.
    let symbols = symcache.lookup(0xd7d).collect::<Vec<_>>();
    assert_eq!(symbols.len(), 2);
    let name = symbols[1].function().name();
    assert_eq!(name, "_ZN15google_breakpad18MinidumpFileWriterD2Ev");

    // Test an address where an inlinee's caller has its second line.
    let symbols = symcache.lookup(0x2dd4).collect::<Vec<_>>();
    assert_eq!(symbols.len(), 5);
    let name = symbols[4].function().name();
    assert_eq!(
        name,
        "_ZN15google_breakpad13DynamicImages18GetExecutableImageEv"
    );

    Ok(())
}

/// This tests the fix for the bug described in
/// https://github.com/getsentry/symbolic/issues/647.
#[test]
fn test_lookup_gap_inlinee() -> Result<(), Error> {
    use symbolic_common::{Language, Name, NameMangling};
    use symbolic_debuginfo::{FileInfo, Function, LineInfo};
    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();

    // Manually add a Function of the right shape. A function like this is theoretically possible
    // to encounter in a PDB file, but it requires the call to inlineeA and inlineeB to be on the
    // same line.
    // The interesting aspect about this function is that inlineeB has a gap, inlineeA sits in that
    // gap, and the "parent line" which covers inlineeB's first chunk also covers the gap.

    converter.process_symbolic_function(&Function {
        address: 0x1000,
        size: 0x30,
        name: Name::new("outer", NameMangling::Unmangled, Language::Rust),
        compilation_dir: b"",
        lines: vec![LineInfo {
            address: 0x1000,
            size: Some(0x30),
            file: FileInfo::from_filename(b"main.rs"),
            line: 5,
        }],
        inlinees: vec![
            Function {
                address: 0x1010,
                size: 0x10,
                name: Name::new("inlineeA", NameMangling::Unmangled, Language::Rust),
                compilation_dir: b"",
                lines: vec![LineInfo {
                    address: 0x1010,
                    size: Some(0x10),
                    file: FileInfo::from_filename(b"main.rs"),
                    line: 20,
                }],
                inlinees: vec![],
                inline: true,
            },
            Function {
                address: 0x1000,
                size: 0x30,
                name: Name::new("inlineeB", NameMangling::Unmangled, Language::Rust),
                compilation_dir: b"",
                lines: vec![
                    LineInfo {
                        address: 0x1000,
                        size: Some(0x10),
                        file: FileInfo::from_filename(b"main.rs"),
                        line: 40,
                    },
                    LineInfo {
                        address: 0x1020,
                        size: Some(0x10),
                        file: FileInfo::from_filename(b"main.rs"),
                        line: 42,
                    },
                ],
                inlinees: vec![],
                inline: true,
            },
        ],
        inline: false,
    });
    converter.serialize(&mut buffer)?;
    let symcache = SymCache::parse(&buffer)?;

    // Test that inlineeA at address 0x1010 was not overwritten by the "remaining parent line"
    // after inlineeB's first line was processed.
    let symbols = symcache.lookup(0x1010).collect::<Vec<_>>();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].function().name(), "inlineeA");
    assert_eq!(symbols[1].function().name(), "outer");

    Ok(())
}

#[test]
fn test_undecorate_windows_symbols() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("windows/crash.pdb"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    let symcache = SymCache::parse(&buffer)?;

    let symbols = symcache.lookup(0x3756).collect::<Vec<_>>();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].function().name(), "malloc");

    Ok(())
}

/// Tests that the cache is lenient toward adding additional flags at the end.
#[test]
fn test_trailing_marker() -> Result<(), Error> {
    let buffer = ByteView::open(fixture("macos/crash.dSYM/Contents/Resources/DWARF/crash"))?;
    let object = Object::parse(&buffer)?;

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object)?;
    converter.serialize(&mut Cursor::new(&mut buffer))?;
    buffer.extend(b"WITH_SYMBOLMAP");

    SymCache::parse(&buffer)?;

    Ok(())
}

/// Tests that addresses between functions are unmapped. See
/// <https://github.com/getsentry/symbolic/pull/897>.
#[test]
fn test_lookup_between_functions() {
    // This file is a Python fixture
    let buffer = ByteView::open(
        "../py/tests/res/ext/1.4.1/release/dSYMs/3CD3E3CC-281E-3AF3-84EB-CDE6D56F0559.dSYM/Contents/Resources/DWARF/CrashLibiOS",
    )
    .unwrap();
    let object = Object::parse(&buffer).unwrap();

    let mut buffer = Vec::new();
    let mut converter = SymCacheConverter::new();
    converter.process_object(&object).unwrap();
    converter.serialize(&mut Cursor::new(&mut buffer)).unwrap();
    let symcache = SymCache::parse(&buffer).unwrap();
    // This address is exactly at the end of the function "-[CRLCrashNXPage desc]",
    // which starts at 0x8b0c and has size 0x2c. The next function,
    // "-[CRLCrashStackGuard category]", starts at 0x8b3c.
    //
    // However, there is an entry in the symbol table for "-[CRLCrashNXPage crash]" starting
    // at this address.
    let symbols = symcache.lookup(0x8b38).collect::<Vec<_>>();
    assert_eq!(symbols.len(), 1);

    let function = symbols[0].function();

    assert_eq!(function.name(), "-[CRLCrashNXPage crash]");
    assert_eq!(function.entry_pc(), 0x8b38);
}
