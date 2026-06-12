import './Rows.css'
import '../theme/colors.css'
import Progress from './Progress';
import { formatSize, formatTime, formatStartTime } from '../utils/format';
import { invoke } from '@tauri-apps/api/core';
import { FaCircleInfo, FaFolderOpen, FaPencil, FaCheck, FaRegTrashCan, FaRegCirclePlay, FaRegCirclePause, FaRegCircleXmark, FaCircleMinus, FaRotateLeft, FaLink } from 'react-icons/fa6';
import { useState, useRef } from 'react';


function Rows({element, selectedTab, repairTargetId, setRepairTargetId, repairStatus, setRepairStatus}){

    const [info, setInfo] = useState(false);
    const [editable, setEditable] = useState(false);
    const [validity, setValidity] = useState("hidden");
    const [oldLink, setOldLink] = useState('');

    const textRef = useRef();
    const selector = element.is_selected ? "Row Row-Selected" : "Row";
    const chks = element.is_selected ? "Chk Chk-Selected" : "Chk";
    const inputEditable = editable ? "Info-Link-Text Editable" : "Info-Link-Text-E";

    async function selectRow(id){
        invoke("toggle_one", { id });
    }

    async function selectOne(id){
        invoke("select_one", { id });
    }

    async function openDir(path){
        invoke("open_file_dir", {path: path});
    }

    function toggleInfo(){
        setInfo(oldVal => !oldVal);
    }

    function toggleEditable(){
        setOldLink(element.link);
        if(editable){
           textRef.current.value = element.link;
           setValidity("hidden");
        }
        setEditable(oldVal => !oldVal);
    }

    async function validate(){
        if (oldLink === textRef.current.value.trim() || !editable){
            return;
        }
        setValidity("checking");
        const url = textRef.current.value.trim();
        const info = await invoke('verify_url', {
            url: url,
            cookies: element.cookies || null,
            userAgent: element.user_agent || null,
            referer: element.referer || null,
            headers: element.headers || {}
        }).catch(()=>null);
        if(!info){
            setRepairStatus(null);
            setValidity("invalid");
            return;
        }
        if(!info.size || info.size!=element.total){
            console.log(info.size)
            console.log(element.size)
            setRepairStatus(null);
            setValidity("invalid");
            return;
        } else {
            setRepairStatus(null);
            setValidity("valid")
            invoke('update_download_source', {
                id: element.id,
                source: {
                    url: url,
                    cookies: element.cookies || null,
                    userAgent: element.user_agent || null,
                    referer: element.referer || null,
                    headers: element.headers || {},
                    resume: info.resume_supported,
                    total: info.size || null
                }
            })
            setEditable(false)
        }
    }

    function catchFromBrowser(){
        if (repairTargetId === element.id) {
            setRepairTargetId(null);
            setRepairStatus(null);
            setValidity("hidden");
            return;
        }

        setRepairTargetId(element.id);
        setRepairStatus({
            id: element.id,
            state: "waiting",
            message: "Waiting for the next browser download to replace this link"
        });
        setValidity("waiting");
    }

    function deleteRow(){
        invoke("delete_download", {id: element.id});
    }

    function removeRow(){
        invoke("remove_download", {id: element.id});
    }

    async function resumeRow(){
        await invoke("resume_download", { id: element.id });
    }


    async function pauseRow(){
        await invoke("pause_download", { id: element.id })
    }

    async function cancelRow(){
        await invoke("cancel_download", { id: element.id });
    }

    function eta(){
        const remaining = element.total - element.downloaded;
        const eta_b = remaining / element.speed;
        return formatTime(eta_b);
    }

    console.log(selectedTab.caption)

    return (
        <div className={selector}>
            <div className='Row-Line1'>
                <div className='Chk-Name-Cover'>
                    <div className={chks} onClick={()=>selectRow(element.id)}/>
                    <div className='Icon-Cover' onClick={toggleInfo} ><FaCircleInfo /></div>
                    <div className='Icon-Cover' onClick={()=>openDir(element.location)}><FaFolderOpen /></div>
                    
                    <div className='Icon-Cover' onClick={deleteRow}>
                                    <FaRegTrashCan />
                    </div>
                    <div className='Icon-Cover' onClick={removeRow}>
                                    <FaCircleMinus />
                    </div>
                    {["Paused", "Cancelled", "Failed"].includes(element.state) && (
                        <div className='Icon-Cover' onClick={resumeRow}>
                            {element.state === "Cancelled" ? <FaRotateLeft /> : <FaRegCirclePlay />}
                        </div>
                    )}
                    {element.state === "Downloading" && (
                        <div className='Icon-Cover' onClick={pauseRow}>
                            <FaRegCirclePause />
                        </div>
                    )}
                    {["Downloading", "Paused", "Failed"].includes(element.state) && (
                        <div className='Icon-Cover' onClick={cancelRow}>
                            <FaRegCircleXmark/>
                        </div>
                    )}

                </div>
                <div className='Row-Text'>{element.name}</div>
                <div className='Row-Resume'>Resume:
                    {
                        element.resume ? <div className='Resume-Status-Y'>Yes</div> : <div className='Resume-Status-N'>No</div>
                    }
                </div>
            </div>
                {    
                    !info && <div className='Clickable' onClick={()=>selectOne(element.id)}>
                        <div className='Row-Line2'>
                            <div>{`${(element.downloaded/element.total*100).toFixed(2)}% | ${formatSize(element.downloaded)} of ${formatSize(element.total)}`}</div>
                            <div className='Row-Middle'>{element.state=="Downloading" ? `Speed: ${formatSize(element.speed)}/s` : `Status: ${element.state}`}</div>
                            <div className='Row-End'>{`ETA: ${eta()} | Elapsed: ${element.elapsed}`}</div>
                        </div>
                        <Progress parts={element.parts} total={element.total }/>
                    </div>
                }
                {
                    info && <div className='Info'>
                        <div className='Info-Grid'>
                            <div className='Info-Item'>
                                <span className='Info-Label'>Location:</span>
                                <span className='Info-Value'>{element.location}</span>
                            </div>
                            <div className='Info-Item'>
                                <span className='Info-Label'>Downloaded:</span>
                                <span className='Info-Value'>{formatSize(element.downloaded)}</span>
                            </div>
                            <div className='Info-Item'>
                                <span className='Info-Label'>Total Size:</span>
                                <span className='Info-Value'>{formatSize(element.total)}</span>
                            </div>
                            <div className='Info-Item'>
                                <span className='Info-Label'>Started at:</span>
                                <span className='Info-Value'>{formatStartTime(element.start_time)}</span>
                            </div>
                            <div className='Info-Item'>
                                <span className='Info-Label'>Elapsed:</span>
                                <span className='Info-Value'>{formatTime(element.elapsed)}</span>
                            </div>
                            <div className='Info-Item'>
                                <span className='Info-Label'>Chunks:</span>
                                <span className='Info-Value'>{element.parts ? element.parts.length : 0}</span>
                            </div>
                        </div>
                        <div className='Info-Link'>           
                            <span className='Info-Label'>Link:</span>
                            <input type="url" defaultValue={element.link} disabled={!editable} className={inputEditable} ref={textRef}/>
                            <div className='Info-Button' onClick={toggleEditable}><FaPencil /></div>
                            <div className='Info-Button' onClick={validate}><FaCheck /></div>
                            <div className={`Info-Button${repairTargetId === element.id ? ' active' : ''}`} onClick={catchFromBrowser} title="Catch replacement link from browser"><FaLink /></div>
                        </div>
                        {(validity != "hidden" || repairStatus?.id === element.id) && <div className={`Info-Validity ${repairStatus?.id === element.id ? repairStatus.state : validity}`}>
                            {validity === "invalid" && "Invalid Link for this download"}
                            {validity === "checking" && "Checking link..."}
                            {repairStatus?.id === element.id ? repairStatus.message : validity === "valid" && "Replacement link saved"}
                            {repairStatus?.id !== element.id && validity === "waiting" && "Waiting for the next browser download to replace this link"}
                        </div> }
                    </div>
                }
        </div>
    );
}

export default Rows;
