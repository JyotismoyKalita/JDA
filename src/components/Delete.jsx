import './Delete.css'
import { invoke } from "@tauri-apps/api/core";


function Delete({setDeleteScreen}){

    async function deleteRow(deleteFile){
        invoke("delete_selected", {deleteFile: deleteFile});
        setDeleteScreen(false);
    }

    return (
        <div className='Delete BG-Primary Primary'>
            <div className='Delete-Row Primary'>
                Do you want to delete associated file if any?
            </div>
            <div className='Delete-Last Primary'>
                <div className='Delete-Button Primary BG-Quarternary' onClick={()=>deleteRow(true)}>
                    Yes
                </div>
                <div className='Delete-Button Primary BG-Quarternary' onClick={()=>deleteRow(false)}>
                    No
                </div>
                <div className='Delete-Button Primary BG-Quarternary' onClick={()=>setDeleteScreen(false)}>
                    Cancel
                </div>
            </div>
        </div>
    );
}

export default Delete;