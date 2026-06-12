import './MainArea.css'
import RowList from '../components/RowList'

function MainArea({selectedTab, data, repairTargetId, setRepairTargetId, repairStatus, setRepairStatus}){

    const pages = {
        0: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state != "Cancelled")} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>,
        1: <RowList selectedTab={selectedTab}  data={data.filter(item => (item.state == "Downloading" || item.state == "Connecting"))} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>,
        2: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Paused")} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>,
        3: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Completed")} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>,
        4: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Cancelled")} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>,
        5: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Failed")} repairTargetId={repairTargetId} setRepairTargetId={setRepairTargetId} repairStatus={repairStatus} setRepairStatus={setRepairStatus}/>
    }

    return (
        <div className='MainArea'>
            {pages[selectedTab.id]}
        </div>
    );
}

export default MainArea;
