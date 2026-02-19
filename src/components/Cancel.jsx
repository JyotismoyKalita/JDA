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
        <div className='Cancel BG-Primary Primary'>
            <div className='Cancel-Row Primary'>
                Are you sure you want to loose progress of uncompleted downloads (if any) in the selections?
            </div>
            <div className='Cancel-Last Primary'>
                <div className='Cancel-Button Primary BG-Quarternary' onClick={cancelRow}>
                    Yes
                </div>
                <div className='Cancel-Button Primary BG-Quarternary' onClick={()=>setCancelScreen(false)}>
                    No
                </div>
            </div>
        </div>
    );
}

export default Cancel;