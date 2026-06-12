import './Add.css'
import { FaRegPaste, FaFolderOpen } from 'react-icons/fa6';
import { useRef } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { v4 as uuidv4 } from "uuid";
import { useState, useEffect } from 'react';
import {formatSize} from '../utils/format'


function Add({setAddScreen, txt, setTxt, serverPayload}){

    const [resumeSupported, setResumeSupported] = useState(false);
    const [size, setSize] = useState(0);
    const [cookies, setCookies] = useState("");
    const [userAgent, setUserAgent] = useState("");
    const [referer, setReferer] = useState("");

    const linkRef = useRef();
    const nameRef = useRef();
    const locationRef = useRef();
    const maxConnectionRef = useRef();
    const verifyRef = useRef();

    async function pasteText() {
        const text = await invoke("paste").catch(() => "");
        const el = linkRef.current;
        el.value = text;
        el.focus();
    }

    async function verify() {
        verifyRef.current.style.backgroundColor = "grey";
        const info = await invoke("verify_url", {
            url: linkRef.current.value.trim(),
            cookies: cookies.length > 0 ? cookies : null,
            userAgent: userAgent.length > 0 ? userAgent : null,
            referer: referer.length > 0 ? referer : null
        }).catch(()=>{
            verifyRef.current.style.backgroundColor = "";
        });
        if(!info){
            verifyRef.current.style.backgroundColor = "";
            return;
        }
        const el = nameRef.current;
        el.value = info.filename ?? "file";
        setResumeSupported(info.resume_supported)
        setSize(info.size);
        el.focus();
        verifyRef.current.style.backgroundColor = "";
        if(!info.resume_supported) { 
            maxConnectionRef.current.max = 1;
            maxConnectionRef.current.value = 1;
        } else {
            maxConnectionRef.current.max = 32;
            maxConnectionRef.current.value = 16;
        }
    }

    async function choose(){
        const text = await invoke("pick_folder");
        const el = locationRef.current;
        el.value = text;
        el.focus();
    }

    async function start(state){

        if(!(linkRef.current.value.length > 0 && nameRef.current.value.length > 0 && locationRef.current.value.length > 0
        )){
            return;
        }
        const threads = resumeSupported ?
        Math.max(1, Math.min(32, parseInt(maxConnectionRef.current.value,10) || 1)) : 1;

        const id = uuidv4();
        const newDownload = {
            id: id,
            link: linkRef.current.value,
            name: nameRef.current.value,
            location: locationRef.current.value,
            resume: resumeSupported,
            downloaded: 0,
            total: size ?? 0,
            start_time: null,
            elapsed: 0,
            speed: 0,
            speeds: Array(Number(threads)).fill(0),
            state: state,
            is_selected: false,
            connections: Array(1).fill(0),
            cookies: cookies.length > 0 ? cookies : null,
            user_agent: userAgent.length > 0 ? userAgent : null,
            referer: referer.length > 0 ? referer : null
        }

        await invoke("add_download", {
            download: newDownload
        });
        if(state==="Downloading"){
            await invoke("resume_download", { id: id });
        }
        setAddScreen(false);
    }

    useEffect(() => {
        if (serverPayload) {
            setTimeout(() => {
                if (linkRef.current) linkRef.current.value = serverPayload.url || "";
                if (nameRef.current) nameRef.current.value = serverPayload.name || "";
                setSize(serverPayload.size || 0);
                setResumeSupported(serverPayload.resume === "true");
                setCookies(serverPayload.cookie || "");
                setUserAgent(serverPayload.userAgent || "");
                setReferer(serverPayload.referer || "");
                if (serverPayload.url) {
                    verify();
                }
            }, 100);
            return;
        }

        if (txt && txt.includes('?')) {
            try {
                const queryString = txt.split('?')[1];
                
                const params = new URLSearchParams(queryString);

                const cleanUrl = params.get("url") || "";
                const fileName = params.get("name") || "";
                const fileSize = parseInt(params.get("size") || "0");
                const resume = params.get("resume") === "true";

                const cookieStr = params.get("cookie") || "";
                const uaStr = params.get("userAgent") || "";

                setTimeout(() => {
                    if (linkRef.current) linkRef.current.value = cleanUrl;
                    if (nameRef.current) nameRef.current.value = fileName;
                    setSize(fileSize);
                    setResumeSupported(resume);
                    setCookies(cookieStr);
                    setUserAgent(uaStr);
                    if (cleanUrl) {
                        verify();
                    }
                    setTxt(""); 
                }, 100);

            } catch (e) {
                console.error("Parsing failed:", e);
            }
        }
    }, [txt, setTxt]);

    return (
        <div className='Add BG-Primary Primary'>
            <div className='Add-Row'>
                Link:
                <input type="url" className='Add-Text BG-Secondary Primary' ref={linkRef}/>
                <div className="BG-Quarternary Add-Button" onClick={verify} ref={verifyRef}>Verify</div>
                <div className="BG-Quarternary Add-Button" onClick={pasteText}>
                    <FaRegPaste />
                </div>
            </div>
            <div className='Add-Row'>
                Name:
                <input type="text" className='Add-Text BG-Secondary Primary' ref={nameRef}/>
            </div>
            <div className='Add-Row'>
                Location:
                <input type="text" className='Add-Text BG-Secondary Primary' ref={locationRef} defaultValue={"Downloads"}/>
                <div className="BG-Quarternary Add-Button" onClick={choose}>
                    <FaFolderOpen />
                </div>
            </div>
            <div className='Add-Row'>
                Max Connections:
                <input type="number" min="1" max="32" defaultValue={16}  className='Add-Text BG-Secondary Primary' ref={maxConnectionRef}/>
            </div>
            <div className='Add-Row'>
                <div>Resume Support: {resumeSupported ? "Yes" : "No"}</div>
                <div>File Size: {formatSize(size)}</div>
            </div>
            <div className='Add-Row Add-Last'>
                <div className="BG-Quarternary Add-Button" onClick={()=>start("Paused")}>Add</div>
                <div className="BG-Quarternary Add-Button" onClick={()=>start("Downloading")}>Start</div>
                <div className="BG-Quarternary Add-Button" onClick={()=>setAddScreen(false)}>Cancel</div>
            </div>
            
        </div>
    );
}

export default Add;