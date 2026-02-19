import './Rows.css'
import '../theme/colors.css'
import Progress from './Progress';
import { formatSize, formatTime, formatStartTime } from '../utils/format';
import { invoke } from '@tauri-apps/api/core';
import { FaCircleInfo, FaFolderOpen, FaPencil, FaCheck, FaRegTrashCan, FaRegCirclePlay, FaRegCirclePause, FaRegCircleXmark, FaCircleMinus, FaRotateLeft } from 'react-icons/fa6';
import { useState, useRef } from 'react';


function Rows({element, selectedTab}){

    const [info, setInfo] = useState(false);
    const [editable, setEditable] = useState(false);
    const [validity, setValidity] = useState("hidden");
    const [oldLink, setOldLink] = useState('');

    const textRef = useRef();
    const selector = element.is_selected ? "Row Primary Row-Selected BG-Primary" : "Row Primary BG-Primary";
    const chks = element.is_selected ? "Chk BG-Quarternary" : "Chk";
    const inputEditable = editable ? "Info-Link-Text  Primary BG-Secondary" : "Info-Link-Text BG-Secondary";

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
        const info = await invoke('verify_url', {url: url}).catch(()=>null);
        if(!info){
            setValidity("invalid");
            return;
        }
        if(!info.size || info.size!=element.total){
            console.log(info.size)
            console.log(element.size)
            setValidity("invalid");
            return;
        } else {
            setValidity("valid")
            invoke('change_link', {id: element.id, url: url})
            setEditable(false)
        }
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
                    <div className='Icon-Cover' onClick={resumeRow}>
                        {selectedTab.caption === "Cancelled" ? <FaRotateLeft /> : <FaRegCirclePlay />}
                    </div>
                    <div className='Icon-Cover' onClick={pauseRow}>
                        <FaRegCirclePause />
                    </div>
                    <div className='Icon-Cover' onClick={cancelRow}>
                        <FaRegCircleXmark/>
                    </div>

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
                        <Progress connections={element.connections} total={element.total }/>
                    </div>
                }
                {
                    info && <div className='Info'>
                        <div>
                            Location: {element.location}
                        </div>
                        <div>
                            Downloaded: {formatSize(element.downloaded)}
                        </div>
                        <div>
                            Total: {formatSize(element.total)}
                        </div>
                        <div>
                            Started at: {formatStartTime(element.start_time)}
                        </div>
                        <div>
                            Elapsed: {formatTime(element.elapsed)}
                        </div>
                        <div>Connections: {element.connections.length}</div>
                        <div className='Info-Link'>           
                            <div>Link:</div>
                            <input type="url" defaultValue={element.link} disabled={!editable} className={inputEditable} ref={textRef}/>
                            <div className='Info-Button' onClick={toggleEditable}><FaPencil /></div>
                            <div className='Info-Button' onClick={validate}><FaCheck /></div>
                        </div>
                        {validity != "hidden" && <div className='Secondary'>
                            {validity === "invalid" && "Invalid Link for this download"}
                            {validity === "checking" && "Checking link..."}
                        </div> }
                    </div>
                }
        </div>
    );
}

export default Rows;