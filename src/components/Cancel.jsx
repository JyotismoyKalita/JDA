import './Cancel.css'
import { invoke } from "@tauri-apps/api/core";


function Cancel({setCancelScreen, data}){

    async function cancelRow(){
        const selected = data.filter(
            row => row.state !== "Completed" && row.is_selected
        );

        for (const row of selected){
            await invoke("cancel_download", { id: row.id });
        }
        setCancelScreen(false);
    }

    return (
        <div className="Add-Overlay" onClick={(e) => {if(e.target.className === 'Add-Overlay') setCancelScreen(false)}}>
            <div className='Add' style={{ width: '400px', textAlign: 'center' }}>
                <div className='Add-Row' style={{ justifyContent: 'center', fontSize: '16px', color: 'var(--text-primary)', marginBottom: '32px' }}>
                    Are you sure you want to lose progress of uncompleted downloads (if any) in the selections?
                </div>
                <div className='Add-Row Add-Last'>
                    <div className='Add-Button danger-btn' onClick={cancelRow}>
                        Yes, Cancel
                    </div>
                    <div className='Add-Button' onClick={()=>setCancelScreen(false)}>
                        No
                    </div>
                </div>
            </div>
        </div>
    );
}

export default Cancel;