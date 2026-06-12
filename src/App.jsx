import './App.css'
import TitleBar from './components/TitleBar'
import TopBar from './components/TopBar'
import TabBar from './components/TabBar'
import MainArea from './Pages/MainArea'
import './theme/colors.css'
import { useState, useEffect, useMemo, useRef} from 'react'
import Add from './components/Add'
import Delete from './components/Delete'
import Cancel from './components/Cancel'
import { invoke, Channel } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event';
import { getCurrent } from '@tauri-apps/plugin-deep-link';


function App() {

  const [items, setItems] = useState([
    {id: 0, caption: "All", isSelected: 1},  
    {id: 1, caption: "Downloading", isSelected: 0},
    {id: 2, caption: "Paused", isSelected: 0},
    {id: 3, caption: "Completed", isSelected: 0},
    {id: 4, caption: "Cancelled", isSelected: 0},    
    {id: 5, caption: "Failed", isSelected: 0}    
  ]);

  const [data, setData] = useState([]);

  const [addScreen, setAddScreen] = useState(false);
  const [deleteScreen, setDeleteScreen] = useState(false);
  const [cancelScreen, setCancelScreen] = useState(false);
  const [txt, setTxt] = useState('');
  const [repairTargetId, setRepairTargetId] = useState(null);
  const [repairStatus, setRepairStatus] = useState(null);
  const repairTargetIdRef = useRef(null);

  const [loading, setLoading] = useState(true);

  async function loadDownloads(){
    invoke("stream_downloads", {
      channel: new Channel((list) => {
        setData(list);
      setLoading(false);
      })
    });
  }

  const [serverPayload, setServerPayload] = useState(null);

useEffect(() => {
    repairTargetIdRef.current = repairTargetId;
  }, [repairTargetId]);

useEffect(() => {
    let unlistenDeepLink;
    let unlistenServer;

    const setupListener = async () => {
      unlistenDeepLink = await listen('process-deep-link', (event) => {
        const url = event.payload;
        if (url) {
          setTxt(url);
          setServerPayload(null);
          setAddScreen(true);
        }
      });
      
      unlistenServer = await listen('open_add_download_from_server', async (event) => {
          if (!event.payload) {
            return;
          }

          if (repairTargetIdRef.current) {
            const payload = event.payload;
            const targetId = repairTargetIdRef.current;

            try {
              await invoke("update_download_source", {
                id: targetId,
                source: {
                  url: payload.url || "",
                  cookies: payload.cookie || null,
                  userAgent: payload.userAgent || null,
                  referer: payload.referer || null,
                  headers: payload.headers || {},
                  resume: payload.resume === "true",
                  total: payload.size || null
                }
              });
              setRepairStatus({
                id: targetId,
                state: "valid",
                message: "Replacement link saved"
              });
              setRepairTargetId(null);
            } catch (err) {
              setRepairStatus({
                id: targetId,
                state: "invalid",
                message: String(err)
              });
            }
            return;
          }

          setServerPayload(event.payload);
          setTxt(event.payload.url);
          setAddScreen(true);
      });
    };

    const checkInitialUrl = async () => {
      try {
        // Give the OS/Rust a tiny bit of breathing room
        await new Promise(r => setTimeout(r, 500)); 
        
        const urls = await getCurrent();
        if (urls && urls.length > 0) {
          const url = urls[0].replace('jda://', '');
          setTxt(url);
          setAddScreen(true);
        }
      } catch (err) {
        console.error("Permission error or plugin failure:", err);
      }
    };

    setupListener();
    checkInitialUrl();
    loadDownloads();

    const handleContextMenu = (e) => {
      if (!["INPUT", "TEXTAREA"].includes(e.target.tagName)) {
        e.preventDefault();
      }
    };
    window.addEventListener("contextmenu", handleContextMenu);

    return () => {
      window.removeEventListener("contextmenu", handleContextMenu);
      if (unlistenDeepLink) unlistenDeepLink();
      if (unlistenServer) unlistenServer();
    };
  }, []);
  
  function selectTab(clickedItem){
      setItems(oldItems =>
          oldItems.map(item =>
              item.id === clickedItem.id
                  ? { ...item, isSelected: 1 }
                  : { ...item, isSelected: 0 }
          )
      );
  }

  const selectedTab = items.find(item => item.isSelected === 1);

  const allSelected = useMemo(() => {
    const filtered = data.filter(d => selectedTab.caption === "All" || d.state === selectedTab.caption);
    return filtered.length > 0 && filtered.every(d => d.is_selected);
  }, [data, selectedTab]);

  const noneSelected = useMemo(() => {
    const filtered = data.filter(d => selectedTab.caption === "All" || d.state === selectedTab.caption);
    return filtered.length === 0 || filtered.every(d => !d.is_selected);
  }, [data, selectedTab]);

   return (
    <div className='Full BG-Tertiary'>
      <TitleBar />
      <TopBar data={data} selectedTab={selectedTab} allSelected={allSelected} noneSelected={noneSelected}  setAddScreen={setAddScreen} setDeleteScreen={setDeleteScreen} setCancelScreen={setCancelScreen}  />
      <TabBar items={items} selectTab={selectTab} data={data}/>
      {!loading && <MainArea selectedTab={selectedTab} data={data} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>}
      {addScreen && <Add setAddScreen={setAddScreen} txt={txt} setTxt={setTxt} serverPayload={serverPayload} />}
      {deleteScreen && <Delete setDeleteScreen={setDeleteScreen}/>}
      {cancelScreen && <Cancel setCancelScreen={setCancelScreen} data={data}/>}
    </div>
  )
}

export default App
