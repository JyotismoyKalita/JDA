import './RowList.css'
import '../theme/colors.css'
import Rows from './Rows';

function RowList({data, selectedTab, repairTargetId, setRepairTargetId, repairStatus, setRepairStatus}){

    return (
        <div className='RowList'>
            {data.map((element, index)=>{
                return (
                    <Rows key={index} element={element} selectedTab={selectedTab} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>
                );
            })}
        </div>
    );
}

export default RowList;
