// App — mounts design canvas with all artboards
const { DesignCanvas, DCSection, DCArtboard, DCSticky } = window;

function App() {
  const [dark, setDark] = React.useState(false);

  return (
    <>
      {/* Fixed theme toggle (outside canvas) */}
      <div style={{
        position:'fixed', top: 14, right: 14, zIndex: 100,
        display:'flex', gap: 6, alignItems:'center',
        padding: 4, borderRadius: 8,
        background:'rgba(255,255,255,.9)',
        border:'0.5px solid rgba(30,24,14,.1)',
        boxShadow:'0 4px 12px rgba(30,24,14,.08)',
        backdropFilter:'blur(20px)',
      }}>
        {['浅色','深色'].map((t,i)=>(
          <button key={t} onClick={()=>setDark(i===1)} style={{
            padding:'5px 10px', fontSize: 11.5, border:'none',
            borderRadius: 6, cursor:'pointer',
            background: (i===1) === dark ? 'var(--lens)' : 'transparent',
            color: (i===1) === dark ? '#fff' : 'var(--warm-700)',
            fontWeight: (i===1) === dark ? 600 : 500,
            fontFamily:'var(--sf)',
            display:'flex', alignItems:'center', gap: 5,
          }}>
            <Icon name={i===0?'sun':'moon'} size={12}/>{t}
          </button>
        ))}
      </div>

      <DesignCanvas>

        <DCSection id="overview" title="Lenz" subtitle="开源、跨平台的截图翻译工具。macOS 原生质感，温暖中性色 + 天空蓝点缀色。">
          <DCArtboard id="welcome" label="01 · 欢迎页" width={720} height={520}>
            <WelcomeArtboard dark={dark}/>
          </DCArtboard>
          <DCArtboard id="tray" label="02 · 托盘菜单" width={300} height={540}>
            <div style={{ paddingTop: 40 }}>
              <TrayArtboard dark={dark}/>
            </div>
          </DCArtboard>
        </DCSection>

        <DCSection id="settings" title="设置窗口" subtitle="侧边栏 + 五个标签页。「服务」页内用分段控件区分文本翻译与 OCR。">
          <DCArtboard id="services" label="服务 · 文本翻译" width={860} height={580}>
            <SettingsArtboard dark={dark} initialTab="services" initialSub="translate"/>
          </DCArtboard>
          <DCArtboard id="general" label="通用" width={860} height={580}>
            <SettingsArtboard dark={dark} initialTab="general"/>
          </DCArtboard>
          <DCArtboard id="shortcuts" label="快捷键" width={860} height={580}>
            <SettingsArtboard dark={dark} initialTab="shortcuts"/>
          </DCArtboard>
          <DCArtboard id="ocr" label="服务 · OCR" width={860} height={580}>
            <SettingsArtboard dark={dark} initialTab="services" initialSub="ocr"/>
          </DCArtboard>
          <DCArtboard id="about" label="关于" width={860} height={580}>
            <SettingsArtboard dark={dark} initialTab="about"/>
          </DCArtboard>
        </DCSection>

        <DCSection id="f1" title="F1 · 划词翻译" subtitle="选中任意文本 → 弹框跟随光标。极简外观，快速关闭。">
          <DCArtboard id="f1-popup" label="弹框 — 多引擎 · 可编辑原文" width={560} height={600}>
            <PopupF1Artboard dark={dark}/>
          </DCArtboard>
        </DCSection>

        <DCSection id="f2" title="F2 · 截图翻译" subtitle="十字框选 → OCR → 译文。模式切换让用户无需重新框选即可切换到原图覆盖。">
          <DCArtboard id="f2-capture" label="选区模式" width={640} height={400}>
            <CaptureF2Artboard dark={dark}/>
          </DCArtboard>
          <DCArtboard id="f2-result" label="结果 — OCR 预览 + 可编辑原文 + 引擎列表" width={460} height={760}>
            <PopupF2Artboard dark={dark}/>
          </DCArtboard>
        </DCSection>

        <DCSection id="f3" title="F3 · 原图覆盖翻译" subtitle="核心差异化功能。每个文本块按原始 bbox 渲染译文，鼠标可穿透到下方原像素。">
          <DCArtboard id="f3-overlay" label="在模拟页面上原位翻译" width={860} height={560}>
            <OverlayF3Artboard dark={dark}/>
          </DCArtboard>
        </DCSection>

      </DesignCanvas>
    </>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App/>);
