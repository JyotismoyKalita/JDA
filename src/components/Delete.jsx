import './Delete.css'
import { invoke } from "@tauri-apps/api/core";


function Delete({setDeleteScreen}){

    async function deleteRow(deleteFile){
        invoke("delete_selected", {deleteFile: deleteFile});
        setDeleteScreen(false);
    }

    return (
        <div className="Add-Overlay" onClick={(e) => {if(e.target.className === 'Add-Overlay') setDeleteScreen(false)}}>
            <div className='Add' style={{ width: '400px', textAlign: 'center' }}>
                <div className='Add-Row' style={{ justifyContent: 'center', fontSize: '16px', color: 'var(--text-primary)', marginBottom: '32px' }}>
                    Do you want to delete associated file if any?
                </div>
                <div className='Add-Row Add-Last'>
                    <div className='Add-Button danger-btn' onClick={()=>deleteRow(true)}>
                        Yes
                    </div>
                    <div className='Add-Button primary' onClick={()=>deleteRow(false)}>
                        No
                    </div>
                    <div className='Add-Button' onClick={()=>setDeleteScreen(false)}>
                        Cancel
                    </div>
                </div>
            </div>
        </div>
    );
}

export default Delete;