// Minimal, consistent icon set — 1.5px stroke, 20x20 viewBox, currentColor.
// Stays quiet so it never out-shouts real content.

const Icon = ({ name, size = 16, stroke = 1.5, style, className }) => {
  const p = {
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: stroke,
    strokeLinecap: 'round',
    strokeLinejoin: 'round',
  };
  const paths = {
    // Lens / aperture
    lens: <><circle cx="10" cy="10" r="7" {...p} /><circle cx="10" cy="10" r="3" {...p} /></>,
    // Function family
    selection: <><path d="M4 4h3M13 4h3M16 7v3M16 13v3M13 16h3M4 16h3M4 13v-3M4 10V7" {...p} /><path d="M9 9l2 5 1-2 2-1z" {...p} /></>,
    capture: <><path d="M3 7V5a1 1 0 0 1 1-1h2M14 4h2a1 1 0 0 1 1 1v2M17 13v2a1 1 0 0 1-1 1h-2M6 16H4a1 1 0 0 1-1-1v-2" {...p} /><circle cx="10" cy="10" r="3" {...p} /></>,
    overlay: <><rect x="3" y="3" width="10" height="10" rx="1.5" {...p} /><rect x="7" y="7" width="10" height="10" rx="1.5" {...p} /></>,
    // Nav
    settings: <><circle cx="10" cy="10" r="2.5" {...p} /><path d="M10 2v2M10 16v2M18 10h-2M4 10H2M15.7 4.3l-1.4 1.4M5.7 14.3l-1.4 1.4M15.7 15.7l-1.4-1.4M5.7 5.7L4.3 4.3" {...p} /></>,
    keyboard: <><rect x="2" y="5" width="16" height="10" rx="2" {...p} /><path d="M5 9h.01M8 9h.01M11 9h.01M14 9h.01M6 12h8" {...p} /></>,
    cpu: <><rect x="5" y="5" width="10" height="10" rx="1.5" {...p} /><path d="M8 8h4v4H8z" {...p} /><path d="M8 2v3M12 2v3M8 15v3M12 15v3M2 8h3M2 12h3M15 8h3M15 12h3" {...p} /></>,
    ocr: <><rect x="3" y="3" width="14" height="14" rx="2" {...p} /><path d="M6 8h2M10 8h4M6 12h5M13 12h1" {...p} /></>,
    history: <><path d="M3 10a7 7 0 1 0 2-5" {...p} /><path d="M3 3v4h4" {...p} /><path d="M10 7v3l2 1.5" {...p} /></>,
    info: <><circle cx="10" cy="10" r="7" {...p} /><path d="M10 9v4M10 7v.01" {...p} /></>,
    // UI bits
    close: <path d="M5 5l10 10M15 5L5 15" {...p} />,
    plus: <path d="M10 4v12M4 10h12" {...p} />,
    check: <path d="M4 10l4 4 8-8" {...p} />,
    chevronDown: <path d="M5 8l5 5 5-5" {...p} />,
    chevronRight: <path d="M8 5l5 5-5 5" {...p} />,
    copy: <><rect x="6" y="6" width="10" height="10" rx="1.5" {...p} /><path d="M4 12V5a1 1 0 0 1 1-1h7" {...p} /></>,
    search: <><circle cx="9" cy="9" r="5" {...p} /><path d="M13 13l3 3" {...p} /></>,
    play: <path d="M6 4l10 6-10 6z" {...p} />,
    pause: <><path d="M7 5v10M13 5v10" {...p} /></>,
    moon: <path d="M15 11a5 5 0 0 1-6-6 6 6 0 1 0 6 6z" {...p} />,
    sun: <><circle cx="10" cy="10" r="3" {...p} /><path d="M10 2v2M10 16v2M18 10h-2M4 10H2M15.7 4.3l-1.4 1.4M5.7 14.3l-1.4 1.4M15.7 15.7l-1.4-1.4M5.7 5.7L4.3 4.3" {...p} /></>,
    swap: <><path d="M5 7h10M5 7l3-3M5 7l3 3M15 13H5M15 13l-3-3M15 13l-3 3" {...p} /></>,
    globe: <><circle cx="10" cy="10" r="7" {...p} /><path d="M3 10h14M10 3c2 2 3 4.5 3 7s-1 5-3 7c-2-2-3-4.5-3-7s1-5 3-7z" {...p} /></>,
    dot: <circle cx="10" cy="10" r="2" fill="currentColor" stroke="none" />,
    drag: <><circle cx="7" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="7" cy="10" r="1" fill="currentColor" stroke="none"/><circle cx="7" cy="14" r="1" fill="currentColor" stroke="none"/><circle cx="13" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="13" cy="10" r="1" fill="currentColor" stroke="none"/><circle cx="13" cy="14" r="1" fill="currentColor" stroke="none"/></>,
    external: <><path d="M11 3h6v6M17 3l-7 7M9 5H5a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-4" {...p} /></>,
    trash: <><path d="M4 6h12M8 6V4h4v2M6 6v10a2 2 0 0 0 2 2h4a2 2 0 0 0 2-2V6" {...p} /></>,
    star: <path d="M10 3l2.2 4.6 5 .7-3.6 3.5.9 5-4.5-2.4L5.5 17l.9-5L2.8 8.3l5-.7z" {...p} />,
    command: <path d="M6 6a2 2 0 1 0 2 2V6m0 0h4m0 0v2a2 2 0 1 0 2-2m-2 2v4m0 0v2a2 2 0 1 1-2-2m2 0H8m0 0a2 2 0 1 1-2 2v-2" {...p} />,
    option: <path d="M3 5h4l5 10h5M12 5h5" {...p} />,
    volume: <><path d="M4 8v4h3l4 3V5L7 8H4z" {...p} /><path d="M14 7.5a3.5 3.5 0 0 1 0 5M16 5a6 6 0 0 1 0 10" {...p} /></>,
    insert: <><path d="M10 3v9" {...p} /><path d="M6.5 8.5L10 12l3.5-3.5" {...p} /><path d="M4 16h12" {...p} /></>,
    pin: <><path d="M8 3h4l-.6 1 .3 4 2.3 2v1.4H6V10l2.3-2 .3-4-.6-1z" {...p} /><path d="M10 11.4V17" {...p} /></>,
  };
  return (
    <svg viewBox="0 0 20 20" width={size} height={size} style={style} className={className} aria-hidden="true">
      {paths[name] || null}
    </svg>
  );
};

// Kbd combo — multiple keys
const Kbd = ({ keys, dark }) => (
  <span style={{ display:'inline-flex', gap:3, alignItems:'center' }}>
    {keys.map((k, i) => <span key={i} className={`ln-kbd${dark ? ' dark':''}`}>{k}</span>)}
  </span>
);

// Lens mark — app logo
const LensMark = ({ size = 28 }) => (
  <img
    src="src/assets/logo.png"
    alt="Lenz"
    width={size}
    height={size}
    style={{ display:'block', width: size, height: size, objectFit:'contain', userSelect:'none', WebkitUserDrag:'none' }}
    draggable={false}
  />
);

// Window chrome (macOS)
const WinChrome = ({ title, dark, right }) => (
  <div style={{
    display:'flex', alignItems:'center', justifyContent:'space-between',
    padding:'10px 12px',
    background: dark ? 'rgba(28,26,22,.6)' : 'rgba(250,248,244,.7)',
    borderBottom: `0.5px solid ${dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)'}`,
    fontSize: 13, fontWeight: 500, color: dark ? '#e8e3d7' : 'var(--warm-700)',
    backdropFilter: 'blur(20px)',
  }}>
    <div style={{ display:'flex', gap:8, alignItems:'center' }}>
      <span style={{display:'flex', gap:8}}>
        <i style={{width:12,height:12,borderRadius:'50%',background:'#ff5f57',display:'block',boxShadow:'inset 0 0 0 .5px rgba(0,0,0,.12)'}}/>
        <i style={{width:12,height:12,borderRadius:'50%',background:'#febc2e',display:'block',boxShadow:'inset 0 0 0 .5px rgba(0,0,0,.12)'}}/>
        <i style={{width:12,height:12,borderRadius:'50%',background:'#28c840',display:'block',boxShadow:'inset 0 0 0 .5px rgba(0,0,0,.12)'}}/>
      </span>
    </div>
    <span style={{
      position:'absolute', left:'50%', transform:'translateX(-50%)',
      fontSize: 13, fontWeight: 500,
      color: dark ? '#d6d0c2' : 'var(--warm-700)',
    }}>{title}</span>
    <div>{right}</div>
  </div>
);

Object.assign(window, { Icon, Kbd, LensMark, WinChrome });
