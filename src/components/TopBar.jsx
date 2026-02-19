import './TopBar.css'
import { FaSquarePlus, FaRegTrashCan, FaRegCirclePlay, FaRegCirclePause, FaRegCircleXmark, FaListCheck, FaDoorOpen, FaRotateLeft } from 'react-icons/fa6';
import '../theme/colors.css'
import { invoke } from "@tauri-apps/api/core";

function TopBar({data, selectedTab, allSelected, noneSelected ,setAddScreen, setDeleteScreen, setCancelScreen}){

    function addLink(){
        setAddScreen(oldVal => !oldVal);
    }

    function deleteRow(){
        if (noneSelected){
            return
        }
        setDeleteScreen(oldVal => !oldVal);
    }

    async function resumeRow(){
        if (noneSelected){
            return
        }
        const selected = data.filter(
            row => ((row.state === "Paused" || row.state === "Cancelled" || row.state === "Failed") && row.is_selected)
        );

        for (const row of selected){
            await invoke("resume_download", { id: row.id });
        }
    }


    async function pauseRow(){
        if (noneSelected){
            return
        }
        const selected = data.filter(
            row => row.state === "Downloading" && row.is_selected
        );

        await Promise.all(
            selected.map(row =>
            invoke("pause_download", { id: row.id })
            )
        );
    }

    async function cancelRow(){
        if (noneSelected || selectedTab.caption === "Cancelled"){
            return
        }
        setCancelScreen(oldVal => !oldVal);
    }

    function selectRow(){
        invoke("toggle_select", { tab: selectedTab.caption });
    }

    function quitApp(){
        invoke("quit");
    }

    const coverStyle = "TopBar-Button-Cover Primary";

    const buttonStyle = "TopBar-Button Primary";

    return (
        <div className="TopBar BG-Primary">
            <div className="TopBar-Button-Cover Secondary" onClick={addLink}>
                <FaSquarePlus className="TopBar-Button Secondary" />
                Add Link
            </div>
            <div className={coverStyle} onClick={deleteRow}>
                <FaRegTrashCan className={buttonStyle} />
                Delete
            </div>
            <div className={coverStyle} onClick={resumeRow}>
                {selectedTab.caption === "Cancelled" ? <FaRotateLeft className={buttonStyle} /> : <FaRegCirclePlay className={buttonStyle} />}
                {selectedTab.caption === "Cancelled" ? "Restart" : "Resume"}
            </div>
            <div className={coverStyle} onClick={pauseRow}>
                <FaRegCirclePause className={buttonStyle} />
                Pause
            </div>
            <div className={coverStyle} onClick={cancelRow}>
                <FaRegCircleXmark className={buttonStyle}/>
                Cancel
            </div>
            <div className="TopBar-Button-Cover Primary" onClick={selectRow}>
                <FaListCheck className={allSelected ? "TopBar-Button Secondary" : "TopBar-Button Primary"} />
                Select All
            </div>
            <div className="TopBar-Button-Cover Primary" onClick={quitApp}>
                <FaDoorOpen className= "TopBar-Button Primary" />
                Quit
            </div>
        </div>
    );
}

export default TopBar;