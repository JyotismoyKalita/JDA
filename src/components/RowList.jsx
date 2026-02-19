import './RowList.css'
import '../theme/colors.css'
import Rows from './Rows';

function RowList({data, selectedTab}){

    return (
        <div className='RowList'>
            {data.map((element, index)=>{
                return (
                    <Rows key={index} element={element} selectedTab={selectedTab}/>
                );
            })}
        </div>
    );
}

export default RowList;