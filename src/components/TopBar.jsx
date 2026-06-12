import './TopBar.css'
import { FaSquarePlus, FaRegTrashCan, FaRegCirclePlay, FaRegCirclePause, FaRegCircleXmark, FaListCheck, FaDoorOpen, FaRotateLeft } from 'react-icons/fa6';
import '../theme/colors.css'
import { invoke } from "@tauri-apps/api/core";

function TopBar({data, selectedTab, allSelected, noneSelected ,setAddScreen, setDeleteScreen, setCancelScreen}){

    const hasSelectedToDelete = data.some(row => row.is_selected);
    const hasSelectedToResume = data.some(row => row.is_selected && ["Paused", "Cancelled", "Failed"].includes(row.state));
    const hasSelectedToPause = data.some(row => row.is_selected && row.state === "Downloading");
    const hasSelectedToCancel = data.some(row => row.is_selected && ["Downloading", "Paused", "Failed"].includes(row.state));

    function addLink(){
        setAddScreen(oldVal => !oldVal);
    }

    function deleteRow(){
        if (!hasSelectedToDelete){
            return
        }
        setDeleteScreen(oldVal => !oldVal);
    }

    async function resumeRow(){
        if (!hasSelectedToResume){
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
        if (!hasSelectedToPause){
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
        if (!hasSelectedToCancel){
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

    function handleWheel(e) {
        if (e.deltaY !== 0) {
            e.currentTarget.scrollLeft += e.deltaY;
            e.preventDefault();
        }
    }

    const showResume = ["All", "Paused", "Cancelled", "Failed"].includes(selectedTab.caption);
    const showPause = ["All", "Downloading"].includes(selectedTab.caption);
    const showCancel = ["All", "Downloading", "Paused", "Failed"].includes(selectedTab.caption);

    return (
        <div className="TopBar" onWheel={handleWheel}>
            <div className="TopBar-Button-Cover add-btn" onClick={addLink}>
                <FaSquarePlus className="TopBar-Icon" />
                <span>Add Link</span>
            </div>
            <div className={`TopBar-Button-Cover danger-btn${hasSelectedToDelete ? '' : ' disabled'}`} onClick={deleteRow}>
                <FaRegTrashCan className="TopBar-Icon" />
                <span>Delete</span>
            </div>
            {showResume && (
                <div className={`TopBar-Button-Cover success-btn${hasSelectedToResume ? '' : ' disabled'}`} onClick={resumeRow}>
                    {selectedTab.caption === "Cancelled" ? <FaRotateLeft className="TopBar-Icon" /> : <FaRegCirclePlay className="TopBar-Icon" />}
                    <span>{selectedTab.caption === "Cancelled" ? "Restart" : "Resume"}</span>
                </div>
            )}
            {showPause && (
                <div className={`TopBar-Button-Cover warning-btn${hasSelectedToPause ? '' : ' disabled'}`} onClick={pauseRow}>
                    <FaRegCirclePause className="TopBar-Icon" />
                    <span>Pause</span>
                </div>
            )}
            {showCancel && (
                <div className={`TopBar-Button-Cover danger-btn${hasSelectedToCancel ? '' : ' disabled'}`} onClick={cancelRow}>
                    <FaRegCircleXmark className="TopBar-Icon"/>
                    <span>Cancel</span>
                </div>
            )}
            <div className="spacer"></div>
            <div className={`TopBar-Button-Cover default-btn${allSelected ? ' active' : ''}`} onClick={selectRow}>
                <FaListCheck className="TopBar-Icon" />
                <span>Select All</span>
            </div>
            <div className="TopBar-Button-Cover default-btn" onClick={quitApp}>
                <FaDoorOpen className="TopBar-Icon" />
                <span>Quit</span>
            </div>
        </div>
    );
}

export default TopBar;