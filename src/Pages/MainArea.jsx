import './MainArea.css'
import RowList from '../components/RowList'

function MainArea({selectedTab, data}){

    const pages = {
        0: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state != "Cancelled")}/>,
        1: <RowList selectedTab={selectedTab}  data={data.filter(item => (item.state == "Downloading" || item.state == "Connecting"))}/>,
        2: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Paused")}/>,
        3: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Completed")}/>,
        4: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Cancelled")}/>,
        5: <RowList selectedTab={selectedTab}  data={data.filter(item => item.state == "Failed")}/>
    }

    return (
        <div className='MainArea'>
            {pages[selectedTab.id]}
        </div>
    );
}

export default MainArea;